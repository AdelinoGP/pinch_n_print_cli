#!/usr/bin/env python3
"""Apply the t91 probe to the host sources (attribution only, never committed).

The probe adds:

  crates/slicer-wasm-host/src/exact_z_query.rs
      counters for query calls, cache misses, geometry-term builds and the
      microseconds spent inside a miss; a `t91_probe_report()` printer.
  crates/slicer-wasm-host/src/support_aggregation.rs
      per-entry `validate_entry` timing, accumulated into the same report.

Two variants exist because the baseline tree has no memo:

  baseline  : HEAD's `exact_z_query.rs`; every cache miss builds the terms
  candidate : the t91 memo; the terms are built once per object

`build-probe.ps1` calls this, builds one arm, and restores the tree.
"""

from __future__ import annotations

import argparse
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
QUERY = REPO / "crates/slicer-wasm-host/src/exact_z_query.rs"
AGGREGATION = REPO / "crates/slicer-wasm-host/src/support_aggregation.rs"
ORDERING = "std::sync::atomic::Ordering::Relaxed"

STATICS = """pub static T91_QUERIES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
pub static T91_MISSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static T91_BUILDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static T91_US_QUERY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static T91_VALIDATE_CALLS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
pub static T91_US_VALIDATE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Probe-only report; prints one line when `PNP_T91_PROBE` is set.
pub fn t91_probe_report() {
    if std::env::var_os("PNP_T91_PROBE").is_none() {
        return;
    }
    let n = |v: &std::sync::atomic::AtomicU64| v.load(std::sync::atomic::Ordering::Relaxed);
    eprintln!(
        "T91PROBE queries={} misses={} builds={} query_ms={:.1} validate_calls={} validate_ms={:.1}",
        n(&T91_QUERIES),
        n(&T91_MISSES),
        n(&T91_BUILDS),
        n(&T91_US_QUERY) as f64 / 1000.0,
        n(&T91_VALIDATE_CALLS),
        n(&T91_US_VALIDATE) as f64 / 1000.0
    );
}

"""

QUERY_COUNT = "        T91_QUERIES.fetch_add(1, " + ORDERING + ");\n"

MISS_AND_TIMER = """        T91_MISSES.fetch_add(1, ORDERING);
        let t91_query_start = std::time::Instant::now();
""".replace("ORDERING", ORDERING)

QUERY_TIMER_END = """        T91_US_QUERY.fetch_add(
            t91_query_start.elapsed().as_micros() as u64,
            ORDERING,
        );
""".replace("ORDERING", ORDERING)

BUILD_COUNT = "        T91_BUILDS.fetch_add(1, " + ORDERING + ");\n"

VALIDATE_PROBE = """        let t91_validate_start = std::time::Instant::now();
        let t91_verdict = validate_entry(&entry, input.exact_z);
        crate::exact_z_query::T91_VALIDATE_CALLS
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        crate::exact_z_query::T91_US_VALIDATE.fetch_add(
            t91_validate_start.elapsed().as_micros() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        match t91_verdict {
"""


def expect_once(text: str, anchor: str, label: str) -> None:
    if text.count(anchor) != 1:
        raise SystemExit(
            f"{label}: expected exactly one anchor, found {text.count(anchor)}"
        )


def patch_query(variant: str) -> None:
    text = QUERY.read_text(encoding="utf-8")

    anchor = "/// Immutable result of one normalized support geometry query."
    expect_once(text, anchor, "statics insertion point")
    text = text.replace(anchor, STATICS + anchor)

    anchor = """        if !physical_z_mm.is_finite() {
            return Err("exact-z-support-query: Z must be finite".into());
        }
"""
    expect_once(text, anchor, "finite check")
    text = text.replace(anchor, anchor + QUERY_COUNT)

    anchor = """        {
            return Ok(Arc::clone(result));
        }

"""
    expect_once(text, anchor, "cache-hit block")
    text = text.replace(anchor, anchor + MISS_AND_TIMER)

    anchor = """        self.cache
            .lock()
"""
    expect_once(text, anchor, "query cache insert")
    text = text.replace(anchor, QUERY_TIMER_END + anchor)

    if variant == "baseline":
        anchor = "        let world_mesh = world_mesh(object);\n"
        expect_once(text, anchor, "baseline world mesh")
        text = text.replace(anchor, anchor + BUILD_COUNT)
    else:
        anchor = (
            "                self.geometry_builds.fetch_add(1, Ordering::Relaxed);\n"
        )
        expect_once(text, anchor, "candidate geometry build counter")
        text = text.replace(anchor, BUILD_COUNT + anchor)

    QUERY.write_text(text, encoding="utf-8")


def patch_aggregation() -> None:
    text = AGGREGATION.read_text(encoding="utf-8")

    anchor = """        match validate_entry(&entry, input.exact_z) {
"""
    expect_once(text, anchor, "validate_entry call")
    text = text.replace(anchor, VALIDATE_PROBE)

    anchor = "    union_same_family_entries(&mut result.retained);\n"
    expect_once(text, anchor, "post-loop union")
    text = text.replace(
        anchor, "    crate::exact_z_query::t91_probe_report();\n" + anchor
    )

    AGGREGATION.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["apply"])
    parser.add_argument("--variant", required=True, choices=["baseline", "candidate"])
    args = parser.parse_args()
    patch_query(args.variant)
    patch_aggregation()
    print(f"probe applied ({args.variant} variant)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
