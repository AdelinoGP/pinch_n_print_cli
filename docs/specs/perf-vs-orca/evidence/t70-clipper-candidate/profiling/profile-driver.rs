//! t70 profiling driver (tier 1): per-case allocation volume + timing of the
//! vendored engine on the t69 corpus. Same parsing/execution semantics as the
//! t69 driver (reused verbatim); adds a thread-local counting GlobalAlloc
//! around the engine run so allocation volume is attributed per case.
//!
//! Usage: t70-prof-driver corpus.cases ... > out.jsonl

use clipper2_rust::{
    ClipType, Clipper64, ClipperOffset, EndType, FillRule, JoinType, Paths64, Point64, PolyTree64,
};
use std::cell::Cell;
use std::{collections::BTreeMap, time::Instant};

// ---- thread-local counting allocator ------------------------------------

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static ALLOC_BYTES: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        ALLOCS.with(|c| c.set(c.get() + 1));
        ALLOC_BYTES.with(|c| c.set(c.get() + layout.size() as u64));
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn counters() -> (u64, u64) {
    (ALLOCS.with(Cell::get), ALLOC_BYTES.with(Cell::get))
}
fn reset_counters() {
    ALLOCS.with(|c| c.set(0));
    ALLOC_BYTES.with(|c| c.set(0));
}

// ---- corpus parsing (t69 driver, verbatim) ------------------------------

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
        assert!(self.opcode <= 4, "unknown opcode");
        for id in self.args.keys() {
            assert!(
                if self.opcode == 0 {
                    (1..=5).contains(id) || *id == 7 || *id == 8
                } else {
                    *id == 10 || *id == 7 || *id == 8
                },
                "unexpected argument ID"
            );
        }
        self.flag(7, u64::from(self.opcode != 0));
        self.flag(8, 0);
        if self.opcode == 0 {
            assert!(self.arg(2) <= 3 && self.arg(3) <= 4, "invalid offset enum");
            for id in [1, 4] {
                assert!(f64::from_bits(self.arg(id)).is_finite());
            }
            assert!(f64::from_bits(*self.args.get(&5).unwrap_or(&0)).is_finite());
        } else {
            assert!(self.arg(10) <= 3, "invalid fill rule");
        }
    }
    fn input_verts(&self) -> usize {
        self.subj.iter().map(|p| p.len()).sum::<usize>()
            + self.clip.iter().map(|p| p.len()).sum::<usize>()
    }
    fn run(&self) -> PolyTree64 {
        let mut tree = PolyTree64::new();
        if self.opcode == 0 {
            let mut engine = ClipperOffset::new(
                f64::from_bits(self.arg(4)),
                f64::from_bits(*self.args.get(&5).unwrap_or(&0)),
                self.flag(7, 0),
                self.flag(8, 0),
            );
            let join = [
                JoinType::Square,
                JoinType::Bevel,
                JoinType::Round,
                JoinType::Miter,
            ][self.arg(2) as usize];
            let end = [
                EndType::Polygon,
                EndType::Joined,
                EndType::Butt,
                EndType::Square,
                EndType::Round,
            ][self.arg(3) as usize];
            engine.add_paths(&self.subj, join, end);
            engine.execute_tree(f64::from_bits(self.arg(1)), &mut tree);
            assert_eq!(engine.error_code(), 0, "offset engine failure");
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
        }
        tree
    }
}

fn summary(tree: &PolyTree64) -> (usize, usize, usize, u64) {
    fn collect(tree: &PolyTree64, node: usize, out: &mut Vec<Vec<clipper2_rust::Path64>>) {
        let mut polygon = vec![tree.nodes[node].polygon().clone()];
        for &hole in tree.nodes[node].children() {
            polygon.push(tree.nodes[hole].polygon().clone());
            for &island in tree.nodes[hole].children() {
                collect(tree, island, out);
            }
        }
        out.push(polygon);
    }
    let mut polys = Vec::new();
    for &node in tree.nodes[0].children() {
        collect(tree, node, &mut polys);
    }
    let mut hash = 14695981039346656037u64;
    let mut rings = 0;
    let mut verts = 0;
    for poly in &polys {
        for ring in poly {
            rings += 1;
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
    (polys.len(), rings, verts, hash)
}

fn main() {
    assert!(std::env::args().len() > 1, "usage: driver corpus.cases ...");
    for file in std::env::args().skip(1) {
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
            assert!(
                subj && (clip == (c.opcode != 0)),
                "incorrect geometry records"
            );
            c.validate();
            let input_verts = c.input_verts();
            let expected = summary(&c.run()); // warm up once (hash outside timing)
                                              // Allocation volume: counters cover engine run + tree drop only.
            let mut alloc_count = 0u64;
            let mut alloc_bytes = 0u64;
            {
                reset_counters();
                let tree = c.run();
                let s = summary(&tree);
                assert_eq!(s, expected, "nondeterministic output");
                let (a, b) = counters();
                alloc_count = a;
                alloc_bytes = b;
                std::hint::black_box(&tree);
            }
            let mut times = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                let output = c.run();
                times.push(start.elapsed().as_nanos() as u64);
                assert_eq!(summary(&output), expected, "nondeterministic output");
                std::hint::black_box(&output);
            }
            times.sort();
            println!(
                "{}",
                serde_json::json!({"file":file,"case_index":index,"site":c.site,
                "layer":c.layer,"opcode":c.opcode,"in_verts":input_verts,
                "repeat_times_ns":times,"out_paths":expected.0,
                "out_rings":expected.1,"out_verts":expected.2,
                "out_hash":format!("{:016x}",expected.3),
                "alloc_count":alloc_count,"alloc_bytes":alloc_bytes})
            );
            index += 1;
        }
    }
}
