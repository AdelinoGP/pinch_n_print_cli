#!/usr/bin/env python3
"""Reduce all four t27 A/B batches into one combined reduction artifact.

Thin wrapper around verify-ab.py (which implements the protocol's verify and
negative-control logic). Applies it to each (mode, cell) batch and writes a
combined JSON. Reads rows.csv files only -- launches no slices.

Usage:
  python reduce-ab.py                      # default campaign path + output
  python reduce-ab.py --campaign <dir> --json <out>
"""

import argparse
import importlib.util
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# t27-serial-floor -> evidence -> perf-vs-orca -> specs -> docs -> repo root
REPO_ROOT = HERE.parents[4]

_spec = importlib.util.spec_from_file_location("verify_ab", HERE / "verify-ab.py")
verify_ab = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(verify_ab)

REFERENCE = {
    "benchy": "76eb4e1df3a47abf9a973be4ac99cd6bac41efa9fb65240786251ec0f7cdf6a8",
    "base": "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319",
}
PROTOCOL_PAIRS = 6

BATCHES = [
    ("ordinary", "benchy"),
    ("ordinary", "base"),
    ("accelerated", "benchy"),
    ("accelerated", "base"),
]


def reduce_batch(campaign: Path, mode: str, cell: str):
    rows_path = campaign / "measure" / mode / cell / "rows.csv"
    entry = {"rows_csv": f"measure/{mode}/{cell}/rows.csv"}
    if not rows_path.exists():
        return {**entry, "status": "missing"}
    rows = verify_ab.load_rows(rows_path)
    report = verify_ab.verify(rows, reference_sha=REFERENCE[cell])
    controls = verify_ab.negative_controls(rows, report)
    medians = report["medians"]
    complete = report["pairs"] == PROTOCOL_PAIRS
    if report["errors"]:
        status = "errors"
    elif not complete:
        status = "incomplete"
    else:
        status = "clean"
    return {
        **entry,
        "status": status,
        "measured_rows": report["measured_rows"],
        "pairs": report["pairs"],
        "retained_pairs": medians.get("retained_pairs"),
        "complete_by_protocol": complete,
        "excluded_pairs": report["excluded_pairs"],
        "per_arm_exclusions": report["per_arm_exclusions"],
        "best_cpu_wall_ratio": report["best_cpu_wall_ratio"],
        "wall_delta_median": medians.get("wall_delta_median"),
        "cpu_delta_median": medians.get("cpu_delta_median"),
        "wall_faster_count": medians.get("wall_faster_count"),
        "cpu_faster_count": medians.get("cpu_faster_count"),
        "keep_clause_met": report["keep_clause_met"],
        "output_sha": report["output_sha"],
        "deltas": report["deltas"],
        "notes": report["notes"],
        "errors": report["errors"],
        "negative_controls": {name: ok for name, ok in controls},
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--campaign",
        default=str(REPO_ROOT / "target" / "t27-ab" / "campaign"),
        help="campaign root holding measure/<mode>/<cell>/rows.csv",
    )
    ap.add_argument("--json", default=str(HERE / "t27-ab-reduction.json"))
    args = ap.parse_args()

    campaign = Path(args.campaign)
    out = {
        "candidate": "SupportAnalysis eligibility-predicate parallel pre-pass (host-only)",
        "changed_file": "crates/slicer-runtime/src/builtins/support_analysis_producer.rs",
        "protocol": "protocol-ab.json",
        "reference_sha256": REFERENCE,
        "batches": {},
    }
    any_incomplete = False
    for mode, cell in BATCHES:
        entry = reduce_batch(campaign, mode, cell)
        out["batches"][f"{mode}/{cell}"] = entry
        if entry.get("status") != "clean":
            any_incomplete = True
        print(
            "-" * 72,
        )
        print(f"{mode}/{cell}: {entry.get('status')}")
        if entry.get("status") == "missing":
            continue
        for d in entry["deltas"]:
            print(
                f"  pair {d['pair']}: wall {d['wall_delta']:+.3f}s "
                f"({'F' if d['wall_faster'] else 'S'}) cpu {d['cpu_delta']:+.3f}s "
                f"({'F' if d['cpu_faster'] else 'S'})"
            )
        print(
            f"  medians: wall {entry['wall_delta_median']:+.3f}s "
            f"({entry['wall_faster_count']}/{entry['retained_pairs']}), "
            f"cpu {entry['cpu_delta_median']:+.3f}s "
            f"({entry['cpu_faster_count']}/{entry['retained_pairs']}), "
            f"keep_clause_met={entry['keep_clause_met']}, "
            f"controls_all_caught={all(entry['negative_controls'].values())}"
        )
        if entry["errors"]:
            print("  ERRORS:")
            for e in entry["errors"]:
                print(f"    - {e}")

    Path(args.json).write_text(json.dumps(out, indent=2), encoding="utf-8")
    print("-" * 72)
    print(f"combined reduction written to {args.json}")
    return 1 if any_incomplete else 0


if __name__ == "__main__":
    sys.exit(main())
