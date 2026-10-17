"""t70 tier-1 profile reducer: per-opclass timing sanity + allocation volume.

Reads the raw JSONL produced by t70-prof-driver over the t69 corpus and
renders deterministic tables:
  - per-opclass case medians (ns) and the t69 pristine Rust reference medians
    (engine times, forward pass) for sanity — this harness measures the same
    vendored 1.1.0 source, so medians must be in the same range;
  - allocation volume per case (count, bytes), medians per opclass, and
    per-vertex intensities (alloc count / input vertex, alloc bytes / input
    vertex);
  - the ten most allocation-heavy cases by bytes.

Usage: python profile-reduce.py [raw_jsonl]
Writes profiling/results/tier1.json and prints a markdown digest.
"""

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
DEFAULT_RAW = (
    REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate/raw/profile-tier1.jsonl"
)

OPCODES = {0: "offset", 1: "difference", 2: "union", 3: "intersection", 4: "xor"}
T69_FORWARD_ENGINE_NS = {  # from t69 FINDINGS "Engine times and scaling"
    "difference": 499800.0,
    "intersection": 2307950.0,
    "offset": 6780100.0,
    "union": 104350600.0,
}


def main() -> int:
    raw = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    cases = [json.loads(line) for line in raw.read_text().splitlines() if line.strip()]
    assert len(cases) == 295, f"expected 295 cases, got {len(cases)}"

    by_op = defaultdict(list)
    for c in cases:
        c["median_ns"] = statistics.median(c["repeat_times_ns"])
        c["opname"] = OPCODES[c["opcode"]]
        by_op[c["opname"]].append(c)

    out = {"raw": str(raw), "cases": len(cases), "opclasses": {}}
    print(f"# t70 tier-1 profile ({len(cases)} cases)\n")
    print(
        "| Opclass | Cases | Median ns | t69 pristine fwd median ns | Median allocs/case | Median alloc bytes/case | Median allocs/1k in-verts | Median bytes/1k in-verts |"
    )
    print("|---|---:|---:|---:|---:|---:|---:|---:|")
    for op, rows in sorted(by_op.items()):
        med_ns = statistics.median(r["median_ns"] for r in rows)
        med_allocs = statistics.median(r["alloc_count"] for r in rows)
        med_bytes = statistics.median(r["alloc_bytes"] for r in rows)
        med_allocs_kv = statistics.median(
            r["alloc_count"] / r["in_verts"] * 1000 for r in rows if r["in_verts"]
        )
        med_bytes_kv = statistics.median(
            r["alloc_bytes"] / r["in_verts"] * 1000 for r in rows if r["in_verts"]
        )
        ref = T69_FORWARD_ENGINE_NS.get(op)
        print(
            f"| {op} | {len(rows)} | {med_ns:.0f} | {ref or '—'} | "
            f"{med_allocs:.0f} | {med_bytes:.0f} | {med_allocs_kv:.0f} | {med_bytes_kv:.0f} |"
        )
        out["opclasses"][op] = {
            "cases": len(rows),
            "median_ns": med_ns,
            "t69_forward_reference_ns": ref,
            "median_allocs": med_allocs,
            "median_alloc_bytes": med_bytes,
            "median_allocs_per_1k_in_verts": med_allocs_kv,
            "median_bytes_per_1k_in_verts": med_bytes_kv,
        }

    top = sorted(cases, key=lambda c: c["alloc_bytes"], reverse=True)[:10]
    print("\n## Ten most allocation-heavy cases (bytes)\n")
    print(
        "| File | Case | Site/layer | Opclass | In verts | Median ns | Allocs | Alloc bytes | Bytes/vert |"
    )
    print("|---|---:|---|---|---:|---:|---:|---:|---:|")
    for c in top:
        print(
            f"| {Path(c['file']).name} | {c['case_index']} | {c['site']}/{c['layer']} "
            f"| {c['opname']} | {c['in_verts']} | {c['median_ns']:.0f} | "
            f"{c['alloc_count']} | {c['alloc_bytes']} | "
            f"{c['alloc_bytes'] / max(c['in_verts'], 1):.0f} |"
        )
    out["top_alloc_cases"] = [
        {
            k: c[k]
            for k in (
                "file",
                "case_index",
                "site",
                "layer",
                "opname",
                "in_verts",
                "median_ns",
                "alloc_count",
                "alloc_bytes",
            )
        }
        for c in top
    ]

    # Per-case records for the committed evidence set.
    out["case_medians"] = [
        {
            k: c[k]
            for k in (
                "file",
                "case_index",
                "site",
                "layer",
                "opname",
                "in_verts",
                "out_verts",
                "median_ns",
                "alloc_count",
                "alloc_bytes",
            )
        }
        for c in cases
    ]
    results_dir = HERE / "results"
    results_dir.mkdir(exist_ok=True)
    (results_dir / "tier1.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"\n[t70] reduced results: {results_dir / 'tier1.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
