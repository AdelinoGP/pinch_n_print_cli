"""t70 tier-1 allocation comparison: Rust (vendored) vs C++ (canonical).

Pairs the per-case allocation volumes measured by the two counting drivers on
identical corpus inputs and renders deterministic tables:
  - per-opclass medians of alloc count / alloc bytes, Rust vs C++ and the
    Rust/C++ ratio;
  - the largest volume-ratio cases (Rust allocating far more than C++).

Usage: python profile-compare-alloc.py [rust.jsonl cpp.jsonl]
Writes profiling/results/alloc-compare.json and prints a markdown digest.
"""

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
RAW = REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate/raw"
OPCODES = {0: "offset", 1: "difference", 2: "union", 3: "intersection", 4: "xor"}


def load(path: Path) -> dict:
    rows = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    out = {}
    for r in rows:
        key = (Path(r["file"]).name, r["case_index"])
        assert key not in out, key
        out[key] = r
    return out


def main() -> int:
    rust = load(Path(sys.argv[1]) if len(sys.argv) > 1 else RAW / "profile-tier1.jsonl")
    cpp = load(
        Path(sys.argv[2]) if len(sys.argv) > 2 else RAW / "profile-tier1-cpp.jsonl"
    )
    assert rust.keys() == cpp.keys(), "case populations differ"
    assert len(rust) == 295, len(rust)

    pairs = []
    for key, r in rust.items():
        c = cpp[key]
        # No value-identity assert here: the 46 known t31 clean_collinear
        # Rust-vs-C++ divergences (documented in t69) are expected; the t69
        # campaign already discloses them and this comparison is about
        # allocation volume, not value agreement.
        pairs.append(
            {
                "file": key[0],
                "case_index": key[1],
                "site": r["site"],
                "layer": r["layer"],
                "opname": OPCODES[r["opcode"]],
                "in_verts": r["in_verts"],
                "rust_alloc_count": r["alloc_count"],
                "rust_alloc_bytes": r["alloc_bytes"],
                "cpp_alloc_count": c["alloc_count"],
                "cpp_alloc_bytes": c["alloc_bytes"],
                "rust_median_ns": statistics.median(r["repeat_times_ns"]),
                "cpp_median_ns": statistics.median(c["repeat_times_ns"]),
            }
        )

    by_op = defaultdict(list)
    for p in pairs:
        by_op[p["opname"]].append(p)

    out = {"pairs": pairs}
    print(f"# t70 tier-1 allocation comparison ({len(pairs)} paired cases)\n")
    print(
        "| Opclass | Rust allocs med | C++ allocs med | allocs ratio | Rust bytes med | C++ bytes med | bytes ratio | Rust/C++ time med ratio |"
    )
    print("|---|---:|---:|---:|---:|---:|---:|---:|")
    for op, rows in sorted(by_op.items()):
        ra = statistics.median(r["rust_alloc_count"] for r in rows)
        ca = statistics.median(r["cpp_alloc_count"] for r in rows)
        rb = statistics.median(r["rust_alloc_bytes"] for r in rows)
        cb = statistics.median(r["cpp_alloc_bytes"] for r in rows)
        rt = statistics.median(
            r["rust_median_ns"] / r["cpp_median_ns"]
            for r in rows
            if r["cpp_median_ns"] > 0
        )
        print(
            f"| {op} | {ra:.0f} | {ca:.0f} | {ra / ca:.2f} | {rb:.0f} | {cb:.0f} | {rb / cb:.2f} | {rt:.2f} |"
        )
        out[op] = {
            "rust_alloc_count_median": ra,
            "cpp_alloc_count_median": ca,
            "rust_alloc_bytes_median": rb,
            "cpp_alloc_bytes_median": cb,
            "time_ratio_median": rt,
        }

    top = sorted(
        pairs,
        key=lambda p: p["rust_alloc_bytes"] / max(p["cpp_alloc_bytes"], 1),
        reverse=True,
    )[:12]
    print("\n## Largest Rust/C++ volume-ratio cases\n")
    print(
        "| File | Case | Site/layer | Opclass | Rust bytes | C++ bytes | Bytes ratio | Rust allocs | C++ allocs |"
    )
    print("|---|---:|---|---|---:|---:|---:|---:|---:|")
    for p in top:
        ratio = p["rust_alloc_bytes"] / max(p["cpp_alloc_bytes"], 1)
        print(
            f"| {p['file']} | {p['case_index']} | {p['site']}/{p['layer']} | {p['opname']} | "
            f"{p['rust_alloc_bytes']} | {p['cpp_alloc_bytes']} | {ratio:.1f} | "
            f"{p['rust_alloc_count']} | {p['cpp_alloc_count']} |"
        )
    out["largest_volume_ratio_cases"] = top

    (HERE / "results").mkdir(exist_ok=True)
    (HERE / "results/alloc-compare.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"\n[t70] results: {HERE / 'results/alloc-compare.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
