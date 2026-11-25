#!/usr/bin/env python3
"""t143 reducer: depth probe rows + campaign CSV -> reduced/summary.json.

Inputs (all under the repository root):

  target/t143/depth/rows.jsonl          probe depth rows (run_depth.py)
  target/t143/<batch>/rows.csv          campaign rows (run_t143.ps1)

Only stdlib. The summary is the in-repo artifact `verify.py` reads; the raw
G-code and stderr trees stay gitignored.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

REFERENCE = {
    "benchy-classic-off": "93e21fee6d757d37b3ae3b4efbfa2b24da178004f9a568d6647e2343ae005123",
    "benchy-classic-on": "b3f1ecccc5385f1b807c3418c271bcf57761a183e60c05eefb3294195df3d7a2",
    "benchy-arachne-off": "3510e8761d65cf3da702b9aa04dd7b0fec64ba2b16996e423bfcd46b72c1fd37",
    "benchy-arachne-on": "96cdac85437082377b351640019b3b1efe7208f6b77ada0f5b9de87035b07697",
    "base-classic-off": "ca14d0554e37eddf27c80b5052b583d6fc6eba5cf7bf551d6bb419fd45a78769",
    "base-classic-on": "585c6a8edc2fd1de72dd2a86933fdad25c7ed144662dd80bbc68fb7856e4f629",
    "base-arachne-off": "d8fdc1d990b7763dd592311c333e1f2b0cd7f7beb9885b99abb5c95a7db2cd98",
    "base-arachne-on": "c9fee59a234212e7868bc600572213c47d6961de1f8963bfe1a34a25cf1f7d06",
}


def load_reference_table() -> tuple[dict, dict]:
    """(t139 base set, replacement-fixture benchy baseline)."""
    path = Path(__file__).resolve().parent / "reference.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    return data["t139_base"], data.get("benchy_replacement_fixture", {})


SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]

# t136's measured union wall on base classic-on at depth 4 (probe, 2026-11-12).
# The depth table re-takes it in-session; this is the cross-take check.
T136_UNION_WALL_MS = 10176.2
# Base classic-on's required prepass cut (t124/t128 ledger).
BASE_CLASSIC_ON_REQUIRED_CUT_S = 92.5
# The 0.80 % paired floor (map's per-shape gate).
FLOOR_PCT = 0.80


def load_depth_rows(path: Path) -> list[dict]:
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            rows.append(json.loads(line))
    return rows


def load_campaign_rows(paths: list[Path]) -> list[dict]:
    rows = []
    for path in paths:
        if not path.exists():
            continue
        with path.open(encoding="utf-8", newline="") as f:
            for row in csv.DictReader(f):
                if not row.get("cell"):
                    continue
                row["wall_seconds"] = float(row["wall_seconds"])
                row["cpu_seconds"] = float(row["cpu_seconds"])
                row["pair"] = int(row["pair"])
                rows.append(row)
    return rows


def depth_summary(rows: list[dict]) -> dict:
    table: dict[str, dict[str, dict]] = {}
    for row in rows:
        depth = str(row["depth"])
        cell = row["cell"]
        table.setdefault(depth, {})[cell] = {
            "wall_seconds": row["wall_seconds"],
            "cpu_seconds": row.get("cpu_seconds"),
            "peak_working_set_bytes": row["peak_working_set_bytes"],
            "clip_core_wall_ms": row["clip_core_wall_ms"],
            "union_calls": row["union"]["calls"],
            "union_requests": row["union"]["requests"],
            "union_wall_ms": round(row["union"]["wall_sum_ns"] / 1e6, 3),
            "union_thread_ms": round(row["union"]["thread_ns"] / 1e6, 3),
            "first_per_layer_ms": row.get("first_per_layer_ms"),
            "first_support_layer_ms": row.get("first_support_layer_ms"),
            "sha256": row["sha256"],
            "reference_match": row["reference_match"],
            "exit_code": row["exit_code"],
        }
    return table


def output_summary(rows: list[dict]) -> dict:
    t139_base, benchy_baseline = load_reference_table()
    out: dict[str, dict[str, dict]] = {}
    for row in rows:
        if row["arm"] not in ("baseline-ordinary", "candidate-ordinary"):
            continue
        expected = t139_base.get(row["cell"])
        entry = out.setdefault(row["cell"], {})
        entry[row["arm"]] = {
            "sha256": row["sha256"],
            "matches_t139": (row["sha256"] == expected) if expected else None,
            "gcode_bytes": int(row["gcode_bytes"]),
            "exit_code": int(row["exit_code"]),
        }
        if expected:
            entry["reference_kind"] = "t139"
        else:
            entry["reference_kind"] = "replacement-fixture-baseline"
            entry["recorded_baseline"] = benchy_baseline.get(row["cell"])
    for cell, entry in out.items():
        base = entry.get("baseline-ordinary")
        cand = entry.get("candidate-ordinary")
        entry["candidate_matches_baseline"] = bool(
            base and cand and base["sha256"] == cand["sha256"]
        )
    return out


def paired_summary(rows: list[dict]) -> dict:
    """Interleaved pairs: one baseline and one candidate row per (cell, pair)."""
    by_cell: dict[str, dict[int, dict[str, dict]]] = {}
    for row in rows:
        if row["mode"] != "ab":
            continue
        if row["arm"] not in ("baseline-ordinary", "candidate-ordinary"):
            continue
        by_cell.setdefault(row["cell"], {}).setdefault(row["pair"], {})[row["arm"]] = (
            row
        )

    out: dict[str, dict] = {}
    for cell, pairs in by_cell.items():
        records = []
        for pair in sorted(pairs):
            arms = pairs[pair]
            if "baseline-ordinary" not in arms or "candidate-ordinary" not in arms:
                continue
            base = arms["baseline-ordinary"]
            cand = arms["candidate-ordinary"]
            records.append(
                {
                    "pair": pair,
                    "order": base.get("order", ""),
                    "baseline_wall_s": base["wall_seconds"],
                    "candidate_wall_s": cand["wall_seconds"],
                    "baseline_cpu_s": base["cpu_seconds"],
                    "candidate_cpu_s": cand["cpu_seconds"],
                    "wall_delta_s": round(
                        cand["wall_seconds"] - base["wall_seconds"], 4
                    ),
                    "wall_saved_pct": round(
                        (base["wall_seconds"] - cand["wall_seconds"])
                        / base["wall_seconds"]
                        * 100.0,
                        4,
                    ),
                    "cpu_delta_s": round(cand["cpu_seconds"] - base["cpu_seconds"], 4),
                    "cpu_saved_pct": round(
                        (base["cpu_seconds"] - cand["cpu_seconds"])
                        / base["cpu_seconds"]
                        * 100.0,
                        4,
                    ),
                }
            )
        if not records:
            continue
        base_wall = statistics.median(r["baseline_wall_s"] for r in records)
        cand_wall = statistics.median(r["candidate_wall_s"] for r in records)
        base_cpu = statistics.median(r["baseline_cpu_s"] for r in records)
        cand_cpu = statistics.median(r["candidate_cpu_s"] for r in records)
        med_wall_saved = (base_wall - cand_wall) / base_wall * 100.0
        med_cpu_saved = (base_cpu - cand_cpu) / base_cpu * 100.0
        out[cell] = {
            "pairs": records,
            "pair_count": len(records),
            "baseline_wall_median_s": round(base_wall, 4),
            "candidate_wall_median_s": round(cand_wall, 4),
            "baseline_cpu_median_s": round(base_cpu, 4),
            "candidate_cpu_median_s": round(cand_cpu, 4),
            "wall_saved_pct_median": round(med_wall_saved, 4),
            "cpu_saved_pct_median": round(med_cpu_saved, 4),
            "wall_saved_s_median": round(base_wall - cand_wall, 4),
            "cpu_saved_s_median": round(base_cpu - cand_cpu, 4),
            "pairs_wall_positive": sum(1 for r in records if r["wall_delta_s"] < 0),
            "pairs_cpu_positive": sum(1 for r in records if r["cpu_delta_s"] < 0),
            "floor_multiple": round(med_wall_saved / FLOOR_PCT, 3),
            "gate_pass": med_wall_saved >= FLOOR_PCT and med_cpu_saved >= FLOOR_PCT,
        }
    return out


def null_summary(rows: list[dict], null_arm: str = "baseline-ordinary") -> dict:
    """Same-arm null control: the instrument's own paired noise band.

    The driver writes two rows per (cell, pair) with the same arm name; file
    order is position 1 then position 2. The delta between them is pure noise.
    """
    grouped: dict[tuple[str, int], list[dict]] = {}
    for row in rows:
        if row["mode"] != "ab" or row["arm"] != null_arm:
            continue
        grouped.setdefault((row["cell"], row["pair"]), []).append(row)

    per_cell: dict[str, list[dict]] = {}
    for (cell, pair), entries in grouped.items():
        if len(entries) != 2:
            continue
        first, second = entries
        per_cell.setdefault(cell, []).append(
            {
                "pair": pair,
                "first_wall_s": first["wall_seconds"],
                "second_wall_s": second["wall_seconds"],
                "wall_delta_pct": round(
                    (second["wall_seconds"] - first["wall_seconds"])
                    / first["wall_seconds"]
                    * 100.0,
                    4,
                ),
                "cpu_delta_pct": round(
                    (second["cpu_seconds"] - first["cpu_seconds"])
                    / first["cpu_seconds"]
                    * 100.0,
                    4,
                ),
            }
        )

    out: dict[str, dict] = {}
    for cell, records in per_cell.items():
        if not records:
            continue
        out[cell] = {
            "pairs": records,
            "pair_count": len(records),
            "wall_delta_pct_median": round(
                statistics.median(r["wall_delta_pct"] for r in records), 4
            ),
            "cpu_delta_pct_median": round(
                statistics.median(r["cpu_delta_pct"] for r in records), 4
            ),
            "wall_delta_pct_abs_max": round(
                max(abs(r["wall_delta_pct"]) for r in records), 4
            ),
            "cpu_delta_pct_abs_max": round(
                max(abs(r["cpu_delta_pct"]) for r in records), 4
            ),
        }
    return out


def depth_delta_summary(depth: dict) -> dict:
    """Union-wall and clip-core-wall reduction per cell against depth 4."""
    deltas: dict[str, dict] = {}
    for cell in {c for cells in depth.values() for c in cells}:
        per_cell = {d: cells[cell] for d, cells in depth.items() if cell in cells}
        if "4" not in per_cell or len(per_cell) < 2:
            continue
        base = per_cell["4"]
        entry = {
            "baseline_union_wall_ms": base["union_wall_ms"],
            "baseline_clip_core_wall_ms": base["clip_core_wall_ms"],
            "baseline_peak_working_set_bytes": base["peak_working_set_bytes"],
        }
        for d, value in sorted(per_cell.items(), key=lambda item: int(item[0])):
            if d == "4":
                continue
            entry[f"depth_{d}"] = {
                "union_wall_ms": value["union_wall_ms"],
                "union_saved_ms": round(
                    base["union_wall_ms"] - value["union_wall_ms"], 3
                ),
                "clip_core_wall_ms": value["clip_core_wall_ms"],
                "clip_core_saved_ms": round(
                    base["clip_core_wall_ms"] - value["clip_core_wall_ms"], 3
                ),
                "peak_working_set_bytes": value["peak_working_set_bytes"],
                "peak_working_set_delta_bytes": value["peak_working_set_bytes"]
                - base["peak_working_set_bytes"],
                "first_per_layer_ms": value.get("first_per_layer_ms"),
            }
        deltas[cell] = entry
    return deltas


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[5]
    )
    ap.add_argument(
        "--depth-rows", type=Path, default=Path("target/t143/depth/rows.jsonl")
    )
    ap.add_argument(
        "--campaign", type=Path, nargs="*", default=[Path("target/t143/ab/rows.csv")]
    )
    ap.add_argument(
        "--output-rows", type=Path, default=Path("target/t143/output/rows.csv")
    )
    ap.add_argument("--null-rows", type=Path, default=Path("target/t143/null/rows.csv"))
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()

    root = args.repo_root.resolve()
    out_path = args.out or (
        Path(__file__).resolve().parent / "reduced" / "summary.json"
    )
    out_path.parent.mkdir(parents=True, exist_ok=True)

    depth_rows = load_depth_rows((root / args.depth_rows).resolve())
    campaign_rows = load_campaign_rows([(root / p).resolve() for p in args.campaign])
    output_rows = load_campaign_rows([(root / args.output_rows).resolve()])
    null_rows = load_campaign_rows([(root / args.null_rows).resolve()])

    depth = depth_summary(depth_rows)
    ab = paired_summary(campaign_rows)
    output = output_summary(output_rows)

    t139_base, benchy_baseline = load_reference_table()
    summary = {
        "ticket": "143",
        "reference": {
            "t139_base": t139_base,
            "benchy_replacement_fixture": benchy_baseline,
            "benchy_note": (
                "The 2026-11-12 replacement 3dbenchy.stl is the same model "
                "(240 layers / 48.0 mm) but a different tessellation, so t139's "
                "four benchy hashes are retired. Benchy cells are checked "
                "candidate == baseline on the replacement fixture; base cells "
                "are checked against t139."
            ),
        },
        "floor_pct": FLOOR_PCT,
        "t136_union_wall_ms_at_depth_4": T136_UNION_WALL_MS,
        "base_classic_on_required_cut_s": BASE_CLASSIC_ON_REQUIRED_CUT_S,
        "depth": depth,
        "depth_deltas": depth_delta_summary(depth),
        "output": output,
        "ab": ab,
        "null": null_summary(null_rows),
        "ab_gate": {
            cell: {
                "gate_pass": ab.get(cell, {}).get("gate_pass"),
                "floor_multiple": ab.get(cell, {}).get("floor_multiple"),
            }
            for cell in CELLS
            if cell in ab
        },
        "sources": {
            "depth_rows": str(args.depth_rows),
            "campaign": [str(p) for p in args.campaign],
            "output_rows": str(args.output_rows),
        },
    }
    out_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out_path}")
    for cell in CELLS:
        if cell in ab:
            entry = ab[cell]
            print(
                f"  {cell:22s} wall {entry['wall_saved_pct_median']:+.3f}% CPU "
                f"{entry['cpu_saved_pct_median']:+.3f}% ({entry['pair_count']} pairs, "
                f"{entry['floor_multiple']:.2f}x floor, gate={entry['gate_pass']})"
            )
    for cell, deltas in summary["depth_deltas"].items():
        parts = []
        for key, value in sorted(deltas.items()):
            if key.startswith("depth_"):
                parts.append(
                    f"d{key.split('_')[1]}: union -{value['union_saved_ms']:.0f}ms"
                )
        if parts:
            print(f"  depth {cell:22s} " + " ".join(parts))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
