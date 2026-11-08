//! t70 profiling driver (tier 2): phase-attributed spans and allocations of
//! the INSTRUMENTED scratch twin engine (prof-twin) on the t69 corpus.
//! Instrumented run — never mixed into wall claims (§3.2).
//!
//! Per case: one warm-up + one measured instrumented execution; records the
//! twin's accumulated per-phase spans and the driver-side per-phase allocation
//! buckets (count + bytes), keyed by prof::PHASE_*.
//!
//! Usage: t70-prof2-driver corpus.cases ... > out.jsonl

use clipper2_rust::prof;
use clipper2_rust::{
    ClipType, Clipper64, ClipperOffset, EndType, FillRule, JoinType, Paths64, Point64, PolyTree64,
};
use std::cell::Cell;
use std::{collections::BTreeMap, time::Instant};

thread_local! {
    static ALLOC_COUNT: Cell<[u64; 24]> = const { Cell::new([0; 24]) };
    static ALLOC_BYTES: Cell<[u64; 24]> = const { Cell::new([0; 24]) };
}

struct PhaseCounting;

unsafe impl std::alloc::GlobalAlloc for PhaseCounting {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let phase = prof::current_phase() as usize % 24;
        ALLOC_COUNT.with(|c| {
            let mut a = c.get();
            a[phase] += 1;
            c.set(a);
        });
        ALLOC_BYTES.with(|c| {
            let mut a = c.get();
            a[phase] += layout.size() as u64;
            c.set(a);
        });
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: PhaseCounting = PhaseCounting;

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
            {
                let _s = prof::span(prof::PHASE_TEARDOWN);
                drop(engine);
            }
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
            {
                let _s = prof::span(prof::PHASE_TEARDOWN);
                drop(engine);
            }
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
            let expected = summary(&c.run()); // warm up once
                                              // Measured instrumented run: reset buckets, run once, capture.
            prof::reset_buckets();
            ALLOC_COUNT.with(|c| c.set([0; 24]));
            ALLOC_BYTES.with(|c| c.set([0; 24]));
            let start = Instant::now();
            {
                let tree = c.run();
                let s = summary(&tree);
                assert_eq!(s, expected, "nondeterministic output");
                std::hint::black_box(&tree);
            }
            let elapsed_ns = start.elapsed().as_nanos() as u64;
            let spans = prof::spans();
            let allocs = ALLOC_COUNT.with(Cell::get);
            let bytes = ALLOC_BYTES.with(Cell::get);
            let mut phases = Vec::new();
            for p in 0..24u32 {
                if spans[p as usize] > 0 || allocs[p as usize] > 0 || bytes[p as usize] > 0 {
                    phases.push(serde_json::json!({
                        "phase": p,
                        "ns": spans[p as usize] as u64,
                        "allocs": allocs[p as usize],
                        "bytes": bytes[p as usize],
                    }));
                }
            }
            println!(
                "{}",
                serde_json::json!({"file":file,"case_index":index,"site":c.site,
                "layer":c.layer,"opcode":c.opcode,"in_verts":input_verts,
                "total_ns_instrumented":elapsed_ns,"out_paths":expected.0,
                "out_rings":expected.1,"out_verts":expected.2,
                "out_hash":format!("{:016x}",expected.3),"phases":phases})
            );
            index += 1;
        }
    }
}
