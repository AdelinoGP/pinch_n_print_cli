"""t70 tier-2 reducer: engine-internal cost centers (phase x opclass).

Reads the tier-2 JSONL (instrumented twin) and renders:
  - per-opclass phase tables: median accumulated span ns across cases, and
    median per-case allocation bytes per phase, with share of the case's
    instrumented total;
  - a pooled per-opclass ranking of phases by time and by bytes.

Instrumented numbers are corroboration-only (span + counting-allocator
overhead inflates them); never mixed into wall claims.

Usage: python profile2-reduce.py [raw_jsonl]
Writes profiling/results/tier2.json and prints a markdown digest.
"""

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
DEFAULT_RAW = (
    REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate/raw/profile-tier2.jsonl"
)
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


def main() -> int:
    raw = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    cases = [json.loads(l) for l in raw.read_text().splitlines() if l.strip()]
    assert len(cases) == 295, len(cases)

    by_op = defaultdict(list)
    for c in cases:
        c["opname"] = OPCODES[c["opcode"]]
        phases = {p["phase"]: p for p in c["phases"]}
        c["p"] = phases
        by_op[c["opname"]].append(c)

    out = {"opclasses": {}}
    for op, rows in sorted(by_op.items()):
        print(f"\n## {op} ({len(rows)} cases)\n")
        print(
            "| Phase | Med span ns/case | Share of spans | Med alloc bytes/case | Share of bytes | Med allocs/case |"
        )
        print("|---|---:|---:|---:|---:|---:|")
        all_phases = set()
        for r in rows:
            all_phases |= set(r["p"].keys())
        total_span_med = statistics.median(
            sum(p["ns"] for p in r["phases"]) for r in rows
        )
        total_bytes_med = statistics.median(
            sum(p["bytes"] for p in r["phases"]) for r in rows
        )
        phase_rows = []
        for ph in sorted(all_phases):
            ns_med = statistics.median(r["p"][ph]["ns"] for r in rows if ph in r["p"])
            by_med = statistics.median(
                r["p"][ph]["bytes"] for r in rows if ph in r["p"]
            )
            al_med = statistics.median(
                r["p"][ph]["allocs"] for r in rows if ph in r["p"]
            )
            n_present = sum(1 for r in rows if ph in r["p"])
            phase_rows.append(
                {
                    "phase": ph,
                    "name": PHASE_NAMES.get(ph, "?"),
                    "cases_present": n_present,
                    "median_ns": ns_med,
                    "median_bytes": by_med,
                    "median_allocs": al_med,
                }
            )
            print(
                f"| {ph} {PHASE_NAMES.get(ph, '?')} | {ns_med:,.0f} | "
                f"{ns_med / total_span_med:.1%} | {by_med:,.0f} | "
                f"{by_med / total_bytes_med:.1%} | {al_med:,.0f} |"
            )
        out["opclasses"][op] = {
            "cases": len(rows),
            "median_total_span_ns": total_span_med,
            "median_total_bytes": total_bytes_med,
            "phases": phase_rows,
        }

    (HERE / "results").mkdir(exist_ok=True)
    (HERE / "results/tier2.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"\n[t70] results: {HERE / 'results/tier2.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
