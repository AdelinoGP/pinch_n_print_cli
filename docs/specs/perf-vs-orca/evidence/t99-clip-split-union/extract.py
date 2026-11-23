#!/usr/bin/env python3
"""Reduce the t99 Phase-1 per-call CSVs into `probe-rows.json`.

Reads the two in-repo row sets (`rows-prod-first.csv`, `rows-split-first.csv`)
and their run metadata (`run-*.json`) and writes one aggregate record per run.
`verify.py` re-derives every claim from the same files, so this script is a
convenience for readers, not a trust boundary.

Usage: python extract.py
"""

import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELL_WALL_SECONDS = 73.17  # t87 ordinary base classic supports-off
FLOOR_SHARE = 0.008

RUNS = [
    ("prod-first", "rows-prod-first.csv", "run-prod-first.json"),
    ("split-first", "rows-split-first.csv", "run-split-first.json"),
]


def load_rows(path):
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def main():
    runs = []
    for tag, rows_name, meta_name in RUNS:
        rows = load_rows(HERE / rows_name)
        meta = json.loads((HERE / meta_name).read_text(encoding="utf-8"))
        prod = [float(r["prod_ms"]) for r in rows]
        offsets = [float(r["offset_pieces_ms"]) for r in rows]
        union_ms = [float(r["union_ms"]) for r in rows]
        split = [o + u for o, u in zip(offsets, union_ms)]
        prod_sum = sum(prod)
        offsets_sum = sum(offsets)
        union_sum = sum(union_ms)
        split_sum = offsets_sum + union_sum
        saving = prod_sum - split_sum
        ratio = meta["cpu_wall_ratio"]
        wall_eq_ms = saving / ratio
        runs.append(
            {
                "tag": tag,
                "rows_file": rows_name,
                "run_file": meta_name,
                "id": meta["id"],
                "order": meta["order"],
                "threads": meta["threads"],
                "cpu_wall_ratio": ratio,
                "wall_seconds": meta["wall_seconds"],
                "cpu_seconds": meta["cpu_seconds"],
                "matches_reference": meta["matches_reference"],
                "calls": len(rows),
                "prod_ms": prod_sum,
                "offset_pieces_ms": offsets_sum,
                "union_ms": union_sum,
                "split_ms": split_sum,
                "saving_ms": saving,
                "split_over_prod": split_sum / prod_sum,
                "union_free_over_prod": offsets_sum / prod_sum,
                "wall_eq_ms": wall_eq_ms,
                "share_pct": 100.0 * wall_eq_ms / (CELL_WALL_SECONDS * 1000.0),
                "exact_calls": sum(int(r["exact_equal"]) for r in rows),
                "set_equal_calls": sum(int(r["set_equal"]) for r in rows),
                "diff_polys": sum(int(r["diff_polys"]) for r in rows),
                "median_prod_ms": statistics.median(prod),
                "median_offsets_ms": statistics.median(offsets),
                "median_union_ms": statistics.median(union_ms),
                "median_operand_polys": statistics.median(
                    int(r["operand_polys"]) for r in rows
                ),
                "median_operand_verts": statistics.median(
                    int(r["operand_verts"]) for r in rows
                ),
            }
        )

    payload = {
        "cell": "ordinary base classic supports-off",
        "cell_wall_seconds": CELL_WALL_SECONDS,
        "floor_share_pct": 100.0 * FLOOR_SHARE,
        "floor_ms": FLOOR_SHARE * CELL_WALL_SECONDS * 1000.0,
        "runs": runs,
    }
    (HERE / "probe-rows.json").write_text(
        json.dumps(payload, indent=2) + "\n", encoding="utf-8"
    )
    for run in runs:
        print(
            f"{run['tag']}: {run['calls']} calls, prod {run['prod_ms']:.1f} ms, "
            f"split {run['split_ms']:.1f} ms ({run['split_over_prod']:.4f}x), "
            f"saving {run['share_pct']:.4f}% of cell, "
            f"exact {run['exact_calls']}/{run['calls']}, "
            f"set-equal {run['set_equal_calls']}/{run['calls']}"
        )


if __name__ == "__main__":
    main()
