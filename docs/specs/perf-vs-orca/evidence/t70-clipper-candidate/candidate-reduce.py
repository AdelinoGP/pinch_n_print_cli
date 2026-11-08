"""t70 candidate reducer: value-exactness + paired medians vs pristine.

Pairs the pristine and candidate JSONL runs case-by-case:
  - exactness: candidate out_hash/out_paths/out_rings/out_verts must equal
    pristine on EVERY case (native leg);
  - timing: per-opclass medians of the per-case median (5 repeats), the
    candidate/pristine ratio, and the per-opclass case-ratio distribution.

Usage: python candidate-reduce.py <pristine.jsonl> <candidate.jsonl> <label>
Writes candidate/results/<label>.json and prints a markdown digest.
"""

import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
OPCODES = {0: "offset", 1: "difference", 2: "union", 3: "intersection", 4: "xor"}


def load(path: str) -> dict:
    rows = [json.loads(l) for l in Path(path).read_text().splitlines() if l.strip()]
    out = {}
    for r in rows:
        key = (Path(r["file"]).name, r["case_index"])
        r["median_ns"] = statistics.median(r["repeat_times_ns"])
        assert key not in out
        out[key] = r
    return out


def main() -> int:
    pristine = load(sys.argv[1])
    candidate = load(sys.argv[2])
    label = sys.argv[3]
    assert pristine.keys() == candidate.keys() and len(pristine) == 295, len(pristine)

    mismatches = []
    for key, p in pristine.items():
        c = candidate[key]
        for field in ("out_hash", "out_paths", "out_rings", "out_verts"):
            if p[field] != c[field]:
                mismatches.append((key, field, p[field], c[field]))

    by_op = defaultdict(list)
    for key, p in pristine.items():
        c = candidate[key]
        op = OPCODES[p["opcode"]]
        by_op[op].append(
            {
                "file": key[0],
                "case_index": key[1],
                "site": p["site"],
                "pristine_ns": p["median_ns"],
                "candidate_ns": c["median_ns"],
            }
        )

    out = {"label": label, "exactness_mismatches": mismatches, "opclasses": {}}
    print(f"# t70 candidate {label}: exactness + paired medians (295 cases)\n")
    if mismatches:
        print(f"**EXACTNESS FAIL: {len(mismatches)} value mismatches**")
        for m in mismatches[:10]:
            print("  -", m)
    else:
        print("**Value-exact on all 295 cases (hash + shape counts), native leg.**")
    print()
    print(
        "| Opclass | Cases | Pristine med ns | Candidate med ns | Cand/Pris med ratio | Case ratio min / med / max |"
    )
    print("|---|---:|---:|---:|---:|---|")
    for op, rows in sorted(by_op.items()):
        pm = statistics.median(r["pristine_ns"] for r in rows)
        cm = statistics.median(r["candidate_ns"] for r in rows)
        ratios = sorted(
            r["candidate_ns"] / r["pristine_ns"] for r in rows if r["pristine_ns"] > 0
        )
        print(
            f"| {op} | {len(rows)} | {pm:,.0f} | {cm:,.0f} | {cm / pm:.3f} | "
            f"{ratios[0]:.2f} / {ratios[len(ratios) // 2]:.2f} / {ratios[-1]:.2f} |"
        )
        out["opclasses"][op] = {
            "cases": len(rows),
            "pristine_median_ns": pm,
            "candidate_median_ns": cm,
            "ratio": cm / pm,
            "case_ratio_min": ratios[0],
            "case_ratio_median": ratios[len(ratios) // 2],
            "case_ratio_max": ratios[-1],
        }

    total_p = statistics.median(
        r["pristine_ns"] for rows in by_op.values() for r in rows
    )
    total_c = statistics.median(
        r["candidate_ns"] for rows in by_op.values() for r in rows
    )
    print(
        f"\nPooled case median: pristine {total_p:,.0f} ns, candidate {total_c:,.0f} ns."
    )

    (HERE / "results").mkdir(exist_ok=True)
    (HERE / f"results/{label}.json").write_text(json.dumps(out, indent=2) + "\n")
    print(f"[t70] results: {HERE / 'results' / (label + '.json')}")
    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
