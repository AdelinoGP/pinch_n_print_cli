"""t70 tier-2 phase comparison: Rust (prof-twin) vs C++ (cpp-twin), per phase.

Pairs per-case per-phase span medians by opclass and renders the Rust/C++
ratio per phase — the engine-internal cost-center attribution the t69
headroom verdict deliberately did not measure. Instrumented numbers are
corroboration-only; never mixed into wall claims.

Usage: python profile2-compare.py [rust.jsonl cpp.jsonl]
Writes profiling/results/tier2-compare.json and prints a markdown digest.
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
PHASE_NAMES = {
    0: "other/untagged",
    1: "add_path",
    2: "insert_local_minima_into_ael",
    3: "do_horizontal",
    4: "build_intersect_list",
    5: "process_intersect_list",
    6: "do_top_of_scanbeam",
    7: "convert_horz_segs_to_joins",
    8: "process_horz_joins",
    9: "reset/clean_up",
    10: "clean_collinear",
    11: "build_result (tree/paths)",
    12: "do_group_offset",
    13: "offset union dispatch",
    14: "engine teardown (drop)",
    15: "union engine total",
    16: "scanline admin",
    17: "list clears",
    18: "do_maxima",
    19: "do_top else (top_x)",
}


def load(path: Path, span_key: str) -> dict:
    rows = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    out = {}
    for r in rows:
        key = (Path(r["file"]).name, r["case_index"])
        out[key] = r
    return out


def main() -> int:
    rust = load(
        Path(sys.argv[1]) if len(sys.argv) > 1 else RAW / "profile-tier2.jsonl",
        "phases",
    )
    cpp = load(
        Path(sys.argv[2]) if len(sys.argv) > 2 else RAW / "profile-tier2-cpp.jsonl",
        "phases",
    )
    assert rust.keys() == cpp.keys() and len(rust) == 295

    pairs = defaultdict(list)  # (opname, phase) -> list of (rust_ns, cpp_ns)
    total_pairs = defaultdict(list)  # opname -> (rust_total, cpp_total)
    for key, r in rust.items():
        c = cpp[key]
        op = OPCODES[r["opcode"]]
        rp = {p["phase"]: p for p in r["phases"]}
        cp = {p["phase"]: p for p in c["phases"]}
        for ph in set(rp) | set(cp):
            pairs[(op, ph)].append(
                (rp.get(ph, {}).get("ns", 0), cp.get(ph, {}).get("ns", 0))
            )
        total_pairs[op].append((r["total_ns_instrumented"], c["total_ns_instrumented"]))

    out = {"phases": {}}
    print("# t70 tier-2 phase comparison (Rust span median / C++ span median)\n")
    for op in sorted({k[0] for k in pairs}):
        print(f"\n## {op}\n")
        print(
            "| Phase | Rust med ns | C++ med ns | Rust/C++ | Present (rust/cpp cases) |"
        )
        print("|---|---:|---:|---:|---|")
        rows = [k for k in sorted(pairs) if k[0] == op]
        for _, ph in rows:
            data = pairs[(op, ph)]
            rm = statistics.median(a for a, _ in data)
            cm = statistics.median(b for _, b in data)
            present = (
                f"{sum(1 for a, _ in data if a > 0)}/{sum(1 for _, b in data if b > 0)}"
            )
            ratio = (rm / cm) if cm > 0 else float("inf")
            print(
                f"| {ph} {PHASE_NAMES.get(ph, '?')} | {rm:,.0f} | {cm:,.0f} | "
                f"{ratio:.2f} | {present} |"
            )
            out["phases"][f"{op}/{ph}"] = {
                "rust_median_ns": rm,
                "cpp_median_ns": cm,
                "ratio": ratio if cm > 0 else None,
            }
        rt = statistics.median(a for a, _ in total_pairs[op])
        ct = statistics.median(b for _, b in total_pairs[op])
        print(
            f"| TOTAL instrumented | {rt:,.0f} | {ct:,.0f} | {rt / ct:.2f} | 295-pool |"
        )
        out["phases"][f"{op}/total"] = {"rust_median_ns": rt, "cpp_median_ns": ct}

    (HERE / "results/tier2-compare.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"\n[t70] results: {HERE / 'results/tier2-compare.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
