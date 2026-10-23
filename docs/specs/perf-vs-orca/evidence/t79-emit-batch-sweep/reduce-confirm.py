#!/usr/bin/env python3
"""t79 pre-gate confirmation reducer.

Reads `confirm/ordinary-base-classic-on/rows.csv` (2 warmups + 2 alternating
pairs, frozen snapshots) and reports the paired wall/CPU deltas of the chosen
constant pair against the committed `4 / 256` baseline. This is a direction
check only; the standing 6-pair campaign is the decision.
"""

from __future__ import annotations

import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def repo_root() -> Path:
    for ancestor in [HERE, *HERE.parents]:
        if (ancestor / ".local-artifacts").is_dir():
            return ancestor
    raise SystemExit("could not locate repo root (.local-artifacts)")


ROOT = (
    repo_root()
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t79-emit-batch-sweep"
    / "confirm"
)
CELL = "ordinary-base-classic-on"


def main() -> int:
    rows_path = ROOT / CELL / "rows.csv"
    if not rows_path.exists():
        raise SystemExit(f"no confirmation rows at {rows_path}")
    rows = list(csv.DictReader(rows_path.open(encoding="utf-8-sig", newline="")))
    measured = [r for r in rows if r["warmup"] == "0"]
    pairs = []
    for index in (1, 2):
        base = next(
            r for r in measured if r["arm"] == "baseline" and int(r["pair"]) == index
        )
        cand = next(
            r for r in measured if r["arm"] == "candidate" and int(r["pair"]) == index
        )
        pairs.append(
            {
                "pair": index,
                "baseline_wall_s": float(base["wall_seconds"]),
                "candidate_wall_s": float(cand["wall_seconds"]),
                "baseline_cpu_s": float(base["cpu_seconds"]),
                "candidate_cpu_s": float(cand["cpu_seconds"]),
                "wall_ratio": float(cand["wall_seconds"]) / float(base["wall_seconds"]),
                "cpu_ratio": float(cand["cpu_seconds"]) / float(base["cpu_seconds"]),
                "sha_equal": base["sha256"] == cand["sha256"],
            }
        )
    wall = statistics.median(p["wall_ratio"] for p in pairs)
    cpu = statistics.median(p["cpu_ratio"] for p in pairs)
    out = {
        "cell": CELL,
        "pairs": pairs,
        "wall_delta_pct_paired_median": (wall - 1.0) * 100.0,
        "cpu_delta_pct_paired_median": (cpu - 1.0) * 100.0,
        "direction": "candidate faster" if wall < 1.0 else "candidate not faster",
        "note": "Direction check only; the standing 6-pair campaign is the decision.",
    }
    (HERE / "confirm.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    for pair in pairs:
        print(
            f"pair {pair['pair']}: wall {pair['baseline_wall_s']:.1f} -> "
            f"{pair['candidate_wall_s']:.1f} s ({100 * (pair['wall_ratio'] - 1):+.2f}%), "
            f"cpu {100 * (pair['cpu_ratio'] - 1):+.2f}%, sha_equal={pair['sha_equal']}"
        )
    print(
        f"paired median wall {out['wall_delta_pct_paired_median']:+.2f}% "
        f"cpu {out['cpu_delta_pct_paired_median']:+.2f}% -> {out['direction']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
