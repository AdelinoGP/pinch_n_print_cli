#!/usr/bin/env python3
"""Reduce raw t158 output / paired A/B / null-control runs into committed JSON."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t158"

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

# Active reference set (t151 eight-cell ordinary SHA-256 reference).
REFERENCE_PATH = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
ACTIVE_REFERENCE = json.loads(REFERENCE_PATH.read_text(encoding="utf-8"))["active_set"]

FLOOR_PCT = 0.80


def parse_stderr_summary(stderr_path: Path) -> dict:
    if not stderr_path.is_file():
        return {"present": False, "ok": False}
    text = stderr_path.read_text(encoding="utf-8", errors="replace").strip()
    if not text:
        return {"present": True, "ok": False, "lines": 0}
    lines = [ln for ln in text.splitlines() if ln.strip()]
    summary = None
    diagnostics = 0
    for ln in lines:
        try:
            obj = json.loads(ln)
        except json.JSONDecodeError:
            continue
        if "diagnostic" in str(obj.get("event", "")).lower():
            diagnostics += 1
        if obj.get("event") == "slice_complete":
            summary = obj
    if summary is None:
        return {"present": True, "ok": False, "lines": len(lines)}
    degraded = bool(summary.get("degraded", True))
    fatal = int(summary.get("fatal_error_count", -1))
    non_fatal = int(summary.get("non_fatal_error_count", -1))
    ok = (
        (not degraded)
        and fatal == 0
        and non_fatal == 0
        and diagnostics == 0
        and summary.get("status") == "ok"
    )
    return {
        "present": True,
        "ok": ok,
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "diagnostic_lines": diagnostics,
    }


def read_csv(path: Path) -> list[dict]:
    with path.open("r", encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def reduce_output(batch: str) -> dict:
    rows = read_csv(RAW / "output" / batch / "rows.csv")
    by_cell: dict[str, dict[str, dict]] = {}
    for r in rows:
        cell = r["cell"]
        arm = r["arm"]
        stderr_info = parse_stderr_summary(Path(r["stderr_path"]))
        by_cell.setdefault(cell, {})[arm] = {
            "exit_code": int(r["exit_code"]),
            "wall_s": float(r["wall_s"]),
            "cpu_s": float(r["cpu_s"]),
            "gcode_sha256": r["gcode_sha256"],
            "gcode_bytes": int(r["gcode_bytes"]),
            "clean": stderr_info,
        }
    out = {}
    for cell in CELLS:
        arms = by_cell[cell]
        base = arms["baseline-ordinary"]
        cand = arms["candidate-ordinary"]
        ref = ACTIVE_REFERENCE[cell]
        out[cell] = {
            "baseline_sha256": base["gcode_sha256"],
            "candidate_sha256": cand["gcode_sha256"],
            "active_reference_sha256": ref,
            "identical": base["gcode_sha256"] == cand["gcode_sha256"],
            "matches_active_reference": cand["gcode_sha256"] == ref,
            "gcode_bytes": cand["gcode_bytes"],
            "baseline_clean": base["clean"]["ok"] and base["exit_code"] == 0,
            "candidate_clean": cand["clean"]["ok"] and cand["exit_code"] == 0,
            "baseline": base,
            "candidate": cand,
        }
    return out


def reduce_ab(batches: list[str]) -> tuple[dict, list[dict]]:
    raw_pairs: dict[str, list[dict]] = {c: [] for c in CELLS}
    batch_metas: list[dict] = []
    pair_offset = 0
    for batch in batches:
        bdir = RAW / "ab" / batch
        meta_path = bdir / "batch_meta.json"
        if meta_path.is_file():
            batch_metas.append(json.loads(meta_path.read_text(encoding="utf-8-sig")))
        rows = read_csv(bdir / "rows.csv")
        grouped: dict[tuple[str, int], dict[str, dict]] = {}
        max_p = 0
        for r in rows:
            p = int(r["pair"])
            max_p = max(max_p, p)
            grouped.setdefault((r["cell"], p), {})[r["side"]] = r
        for (cell, p), sides in sorted(grouped.items()):
            b = sides["baseline"]
            c = sides["candidate"]
            b_clean = parse_stderr_summary(Path(b["stderr_path"]))
            c_clean = parse_stderr_summary(Path(c["stderr_path"]))
            b_wall = float(b["wall_s"])
            c_wall = float(c["wall_s"])
            b_cpu = float(b["cpu_s"])
            c_cpu = float(c["cpu_s"])
            raw_pairs[cell].append(
                {
                    "batch": batch,
                    "pair": pair_offset + p,
                    "order": b["order"],
                    "baseline_wall_s": b_wall,
                    "candidate_wall_s": c_wall,
                    "wall_saving_pct": round((1.0 - c_wall / b_wall) * 100.0, 4),
                    "baseline_cpu_s": b_cpu,
                    "candidate_cpu_s": c_cpu,
                    "cpu_saving_pct": round((1.0 - c_cpu / b_cpu) * 100.0, 4),
                    "baseline_peak_ws_bytes": int(b["peak_ws_bytes"]),
                    "candidate_peak_ws_bytes": int(c["peak_ws_bytes"]),
                    "baseline_sha256": b["gcode_sha256"],
                    "candidate_sha256": c["gcode_sha256"],
                    "baseline_clean": b_clean,
                    "candidate_clean": c_clean,
                    "both_clean": (
                        b_clean["ok"]
                        and c_clean["ok"]
                        and int(b["exit_code"]) == 0
                        and int(c["exit_code"]) == 0
                    ),
                }
            )
        pair_offset += max_p

    summary: dict[str, dict] = {}
    for cell in CELLS:
        plist = raw_pairs[cell]
        if not plist:
            continue
        b_walls = [p["baseline_wall_s"] for p in plist]
        c_walls = [p["candidate_wall_s"] for p in plist]
        b_cpus = [p["baseline_cpu_s"] for p in plist]
        c_cpus = [p["candidate_cpu_s"] for p in plist]
        b_med = statistics.median(b_walls)
        c_med = statistics.median(c_walls)
        b_cpu_med = statistics.median(b_cpus)
        c_cpu_med = statistics.median(c_cpus)
        wall_saving_pct = (1.0 - c_med / b_med) * 100.0
        cpu_saving_pct = (1.0 - c_cpu_med / b_cpu_med) * 100.0
        wins = sum(1 for p in plist if p["candidate_wall_s"] < p["baseline_wall_s"])
        cpu_wins = sum(1 for p in plist if p["candidate_cpu_s"] < p["baseline_cpu_s"])
        shas = {p["baseline_sha256"] for p in plist} | {
            p["candidate_sha256"] for p in plist
        }
        summary[cell] = {
            "pair_count": len(plist),
            "baseline_wall_median_s": round(b_med, 6),
            "candidate_wall_median_s": round(c_med, 6),
            "wall_delta_s": round(c_med - b_med, 6),
            "wall_saving_pct": round(wall_saving_pct, 4),
            "floor_multiple": round(wall_saving_pct / FLOOR_PCT, 2),
            "clears_floor_0p80pct": wall_saving_pct >= FLOOR_PCT,
            "baseline_cpu_median_s": round(b_cpu_med, 6),
            "candidate_cpu_median_s": round(c_cpu_med, 6),
            "cpu_delta_s": round(c_cpu_med - b_cpu_med, 6),
            "cpu_saving_pct": round(cpu_saving_pct, 4),
            "cpu_corroborates": c_cpu_med < b_cpu_med,
            "pairs_candidate_faster": wins,
            "pairs_candidate_lower_cpu": cpu_wins,
            "output_sha256_stable": len(shas) == 1
            and next(iter(shas)) == ACTIVE_REFERENCE[cell],
            "all_runs_clean": all(p["both_clean"] for p in plist),
            "pairs": plist,
        }
    return summary, batch_metas


def reduce_null(batch: str) -> dict:
    bdir = RAW / "ab" / batch
    meta = json.loads((bdir / "batch_meta.json").read_text(encoding="utf-8-sig"))
    rows = read_csv(bdir / "rows.csv")
    grouped: dict[tuple[str, int], dict[str, dict]] = {}
    for r in rows:
        grouped.setdefault((r["cell"], int(r["pair"])), {})[r["side"]] = r
    by_cell: dict[str, list[dict]] = {}
    for (cell, p), sides in sorted(grouped.items()):
        a = sides["baseline"]
        b = sides["baseline2"]
        a_wall = float(a["wall_s"])
        b_wall = float(b["wall_s"])
        a_cpu = float(a["cpu_s"])
        b_cpu = float(b["cpu_s"])
        a_clean = parse_stderr_summary(Path(a["stderr_path"]))
        b_clean = parse_stderr_summary(Path(b["stderr_path"]))
        by_cell.setdefault(cell, []).append(
            {
                "pair": p,
                "order": a["order"],
                "first_wall_s": a_wall,
                "second_wall_s": b_wall,
                "wall_delta_pct": round((b_wall - a_wall) / a_wall * 100.0, 4),
                "first_cpu_s": a_cpu,
                "second_cpu_s": b_cpu,
                "cpu_delta_pct": round((b_cpu - a_cpu) / a_cpu * 100.0, 4),
                "first_sha256": a["gcode_sha256"],
                "second_sha256": b["gcode_sha256"],
                "both_clean": (
                    a_clean["ok"]
                    and b_clean["ok"]
                    and int(a["exit_code"]) == 0
                    and int(b["exit_code"]) == 0
                ),
            }
        )
    cells_out = {}
    all_wall_abs = []
    all_cpu_abs = []
    for cell, plist in by_cell.items():
        w_med = statistics.median([p["wall_delta_pct"] for p in plist])
        c_med = statistics.median([p["cpu_delta_pct"] for p in plist])
        w_max = max(abs(p["wall_delta_pct"]) for p in plist)
        c_max = max(abs(p["cpu_delta_pct"]) for p in plist)
        all_wall_abs.append(w_max)
        all_cpu_abs.append(c_max)
        cells_out[cell] = {
            "pair_count": len(plist),
            "wall_delta_pct_median": round(w_med, 4),
            "cpu_delta_pct_median": round(c_med, 4),
            "wall_abs_max_pct": round(w_max, 4),
            "cpu_abs_max_pct": round(c_max, 4),
            "all_pairs_clean": all(p["both_clean"] for p in plist),
            "pairs": plist,
        }
    return {
        "batch_meta": meta,
        "band_pct": {
            "wall_abs_max_pct": round(max(all_wall_abs), 4),
            "cpu_abs_max_pct": round(max(all_cpu_abs), 4),
            "wall_median_abs_max_pct": round(
                max(abs(v["wall_delta_pct_median"]) for v in cells_out.values()), 4
            ),
            "cpu_median_abs_max_pct": round(
                max(abs(v["cpu_delta_pct_median"]) for v in cells_out.values()), 4
            ),
        },
        "cells": cells_out,
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--output-batch", default="output")
    ap.add_argument("--ab-batches", nargs="+", default=["ab"])
    ap.add_argument("--null-batch", default="null")
    ap.add_argument(
        "--out",
        type=Path,
        default=HERE / "reduced" / "campaign.json",
    )
    args = ap.parse_args()

    manifest = json.loads((RAW / "manifest.json").read_text(encoding="utf-8-sig"))
    output = reduce_output(args.output_batch)
    ab, ab_metas = reduce_ab(args.ab_batches)
    null = reduce_null(args.null_batch)

    gates = {
        "output_identical_8_of_8": all(output[c]["identical"] for c in CELLS),
        "reference_set_unmoved_8_of_8": all(
            output[c]["matches_active_reference"] for c in CELLS
        ),
        "output_runs_clean_8_of_8": all(
            output[c]["baseline_clean"] and output[c]["candidate_clean"] for c in CELLS
        ),
        "timing_floor_8_of_8": all(ab[c]["clears_floor_0p80pct"] for c in CELLS),
        "cpu_corroborates_8_of_8": all(ab[c]["cpu_corroborates"] for c in CELLS),
        "no_cell_regresses": all(ab[c]["wall_saving_pct"] > 0.0 for c in CELLS),
        "ab_output_stable_8_of_8": all(ab[c]["output_sha256_stable"] for c in CELLS),
        "ab_runs_clean_8_of_8": all(ab[c]["all_runs_clean"] for c in CELLS),
        "null_control_complete": len(null["cells"]) == 3
        and all(v["pair_count"] == 3 for v in null["cells"].values()),
        "null_control_clean": all(
            v["all_pairs_clean"] for v in null["cells"].values()
        ),
    }
    verdict = "KEEP" if all(gates.values()) else "DROP"

    payload = {
        "ticket": 158,
        "slug": "t158-needs-support-fast-predicate",
        "floor_pct": FLOOR_PCT,
        "verdict": verdict,
        "gates": gates,
        "drop_cells": [c for c in CELLS if not ab[c]["clears_floor_0p80pct"]],
        "manifest": manifest,
        "active_reference": ACTIVE_REFERENCE,
        "output": output,
        "ab_batch_metas": ab_metas,
        "ab": ab,
        "null": null,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {args.out} (verdict={verdict}, drop_cells={payload['drop_cells']})")


if __name__ == "__main__":
    main()
