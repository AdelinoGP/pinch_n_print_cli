"""Summarize ticket-18 profiled captures without summing overlapping scope totals.

Usage: python docs/specs/perf-vs-orca/evidence/t18-accelerated-residual/summarize.py target/t18
The input JSONL and G-code are scratch; the printed digest is retained in FINDINGS.md.
"""

import json
import sys
from collections import Counter
from pathlib import Path


def capture(path):
    profile = None
    complete = None
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            if not line.startswith("{"):
                continue
            event = json.loads(line)
            if event.get("event") == "profile_summary":
                profile = event["profile"]
            elif event.get("event") == "slice_complete":
                complete = event
    assert profile is not None and complete is not None, path
    classic = next(
        row
        for row in profile["modules"]
        if row["module_id"] == "com.core.classic-perimeters"
    )
    return classic, complete


def sections(path):
    counts = Counter()
    with path.open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            if line.startswith(";TYPE:"):
                counts[line.strip()[6:]] += 1
    return dict(sorted(counts.items()))


def summarize(folder, model):
    baseline, base_end = capture(folder / f"{model}-profile.jsonl")
    probe, probe_end = capture(folder / f"{model}-probe.jsonl")
    assert base_end["status"] == probe_end["status"] == "ok"
    scopes = {
        row["scope"][5:]: row
        for row in probe["scopes"]
        if row["scope"].startswith("t18::")
    }

    def calls(name):
        return scopes.get(name, {}).get("calls", 0)

    def self_fuel(name):
        return scopes.get(name, {}).get("self_fuel", 0)

    def total_fuel(name):
        return scopes.get(name, {}).get("total_fuel", 0)

    indexed = (
        "indexed_distance",
        "indexed_inside",
        "indexed_quartile",
        "indexed_bridge",
    )
    fallbacks = ("fallback_distance", "fallback_quartile", "fallback_bridge")
    reasons = ("linear_fallback", "unsafe_bridge_fallback", "other_fallback")
    parents = ("distance", "quartile", "bridge")
    query = sum(total_fuel(name) for name in parents)
    work = sum(self_fuel(name) for name in indexed)
    scans = sum(self_fuel(name) for name in fallbacks)
    precheck = self_fuel("bridge_arithmetic_check")
    reason_overhead = sum(self_fuel(name) for name in reasons)
    wrapper = sum(self_fuel(name) for name in parents)
    accounted = work + scans + precheck + reason_overhead + wrapper
    assert accounted == query, (model, accounted, query)
    assert calls("quartile") == calls("indexed_quartile") + calls("fallback_quartile")
    assert calls("bridge") == calls("indexed_bridge") + calls("fallback_bridge")
    assert calls("bridge_arithmetic_check") == calls("indexed_bridge") + calls(
        "unsafe_bridge_fallback"
    )
    assert sum(calls(name) for name in reasons) == sum(
        calls(name) for name in fallbacks
    )
    assert calls("distance") == calls("indexed_inside") + calls("fallback_distance")

    outputs = {}
    for suffix, end in (("classic-off", base_end), ("probe", probe_end)):
        gcode = folder / f"{model}-{suffix}.gcode"
        outputs[suffix] = {
            "bytes": gcode.stat().st_size,
            "types": sections(gcode),
            "degraded": end["degraded"],
            "non_fatal_errors": end["non_fatal_error_count"],
        }
    return {
        "fixture": model,
        "outputs": outputs,
        "baseline_classic_fuel": baseline["total_fuel"],
        "probe_classic_fuel": probe["total_fuel"],
        "baseline_classic_self": baseline["self_fuel"],
        "probe_classic_self": probe["self_fuel"],
        "query_fuel_inclusive": query,
        "indexed_internals_including_exact_predicates": work,
        "fallback_scan_fuel": scans,
        "bridge_precheck_fuel": precheck,
        "fallback_reason_overhead": reason_overhead,
        "public_query_wrapper_fuel": wrapper,
        "context_build_fuel": total_fuel("context"),
        "scopes": {
            name: {
                "calls": row["calls"],
                "self_fuel": row["self_fuel"],
                "total_fuel": row["total_fuel"],
            }
            for name, row in sorted(scopes.items())
        },
    }


if __name__ == "__main__":
    folder = Path(sys.argv[1])
    print(
        json.dumps([summarize(folder, model) for model in ("benchy", "base")], indent=2)
    )
