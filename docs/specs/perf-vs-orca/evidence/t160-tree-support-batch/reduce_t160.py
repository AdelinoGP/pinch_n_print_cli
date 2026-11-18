#!/usr/bin/env python3
"""Reduce raw t160 hash / output / paired A/B / null / ceiling runs into JSON."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t160"

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

SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]

FLOOR_PCT = 0.80

REFERENCE_PATH = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
ACTIVE_REFERENCE = json.loads(REFERENCE_PATH.read_text(encoding="utf-8"))["active_set"]


def parse_stderr(stderr_path: Path) -> dict:
    """Parse a slice run's stderr: completion summary, diagnostics, probe lines."""
    if not stderr_path.is_file():
        return {"present": False, "ok": False}
    text = stderr_path.read_text(encoding="utf-8", errors="replace")
    lines = [ln for ln in text.splitlines() if ln.strip()]
    summary = None
    diagnostics = 0
    hash_lines: dict[str, dict[int, dict[str, int]]] = {"batch": {}, "per_layer": {}}
    batch_wall_ms = None
    hash_wall_ms = None
    batch_skipped = False
    for ln in lines:
        if ln.startswith("t160 support_batch skipped"):
            batch_skipped = True
            continue
        if ln.startswith("t160 input_hash "):
            parts = ln.split()
            if "wall_ms=" in ln:
                hash_wall_ms = float(parts[-1].split("=")[1])
                continue
            side = parts[2]
            layer = int(next(p for p in parts if p.startswith("layer=")).split("=")[1])
            fields = {
                p.split("=")[0]: int(p.split("=")[1], 16)
                for p in parts
                if p in ("plan", "view", "paint")
                or p.split("=")[0] in ("plan", "view", "paint")
            }
            hash_lines[side][layer] = fields
            continue
        if ln.startswith("t160 support_batch wall_ms="):
            batch_wall_ms = float(ln.split("wall_ms=")[1].split()[0])
            continue
        if ln.startswith("{"):
            try:
                obj = json.loads(ln)
            except json.JSONDecodeError:
                continue
            if "diagnostic" in str(obj.get("event", "")).lower():
                diagnostics += 1
            if obj.get("event") == "slice_complete":
                summary = obj
    if summary is None:
        return {
            "present": True,
            "ok": False,
            "lines": len(lines),
            "hash": hash_lines,
            "batch_pass_wall_ms": batch_wall_ms,
            "batch_skipped": batch_skipped,
            "hash_pass_wall_ms": hash_wall_ms,
        }
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
        "hash": hash_lines,
        "batch_pass_wall_ms": batch_wall_ms,
        "batch_skipped": batch_skipped,
        "hash_pass_wall_ms": hash_wall_ms,
    }


def read_csv(path: Path) -> list[dict]:
    with path.open("r", encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def reduce_hash(batch: str) -> dict:
    rows = read_csv(RAW / "hash" / batch / "rows.csv")
    cells: dict[str, dict] = {}
    for r in rows:
        cell = r["cell"]
        info = parse_stderr(Path(r["stderr_path"]))
        hashes = info.get("hash", {"batch": {}, "per_layer": {}})
        batch_layers = hashes.get("batch", {})
        per_layers = hashes.get("per_layer", {})
        shared = sorted(set(batch_layers) & set(per_layers))
        mismatches = {"plan": [], "view": [], "paint": []}
        for layer in shared:
            for component in ("plan", "view", "paint"):
                if batch_layers[layer].get(component) != per_layers[layer].get(
                    component
                ):
                    mismatches[component].append(layer)
        cells[cell] = {
            "batch_layers": len(batch_layers),
            "per_layer_layers": len(per_layers),
            "shared_layers": len(shared),
            "missing_from_batch": sorted(set(per_layers) - set(batch_layers))[:20],
            "missing_from_per_layer": sorted(set(batch_layers) - set(per_layers))[:20],
            "mismatch_counts": {k: len(v) for k, v in mismatches.items()},
            "mismatch_layers": {k: v[:20] for k, v in mismatches.items()},
            "match": (
                len(shared) > 0
                and len(shared) == len(batch_layers) == len(per_layers)
                and all(len(v) == 0 for v in mismatches.values())
            ),
            "hash_pass_wall_ms": info.get("hash_pass_wall_ms"),
            "run_clean": bool(info.get("ok")) and int(r["exit_code"]) == 0,
        }
    return {
        "cells": cells,
        "all_cells_match": all(c["match"] for c in cells.values()),
        "all_runs_clean": all(c["run_clean"] for c in cells.values()),
    }


def reduce_output(batch: str) -> dict:
    rows = read_csv(RAW / "output" / batch / "rows.csv")
    by_cell: dict[str, dict[str, dict]] = {}
    for r in rows:
        info = parse_stderr(Path(r["stderr_path"]))
        by_cell.setdefault(r["cell"], {})[r["arm"]] = {
            "exit_code": int(r["exit_code"]),
            "wall_s": float(r["wall_s"]),
            "cpu_s": float(r["cpu_s"]),
            "gcode_sha256": r["gcode_sha256"],
            "gcode_bytes": int(r["gcode_bytes"]),
            "clean": bool(info.get("ok")),
            "batch_pass_wall_ms": info.get("batch_pass_wall_ms"),
        }
    out = {}
    for cell in CELLS:
        arms = by_cell[cell]
        base = arms["baseline"]
        candidate = arms["batch"]
        ref = ACTIVE_REFERENCE[cell]
        out[cell] = {
            "baseline_sha256": base["gcode_sha256"],
            "candidate_sha256": candidate["gcode_sha256"],
            "active_reference_sha256": ref,
            "identical": base["gcode_sha256"] == candidate["gcode_sha256"],
            "matches_active_reference": (
                candidate["gcode_sha256"] == ref and base["gcode_sha256"] == ref
            ),
            "gcode_bytes": candidate["gcode_bytes"],
            "baseline_clean": base["clean"] and base["exit_code"] == 0,
            "candidate_clean": candidate["clean"] and candidate["exit_code"] == 0,
            "batch_pass_wall_ms": candidate["batch_pass_wall_ms"],
        }
    return out


def reduce_ab(batch: str) -> tuple[dict, dict]:
    bdir = RAW / "ab" / batch
    meta = json.loads((bdir / "batch_meta.json").read_text(encoding="utf-8-sig"))
    rows = read_csv(bdir / "rows.csv")
    grouped: dict[tuple[str, int], dict[str, dict]] = {}
    for r in rows:
        grouped.setdefault((r["cell"], int(r["pair"])), {})[r["side"]] = r
    raw_pairs: dict[str, list[dict]] = {c: [] for c in CELLS}
    for (cell, p), sides in sorted(grouped.items()):
        b = sides["baseline"]
        c = sides["candidate"]
        b_info = parse_stderr(Path(b["stderr_path"]))
        c_info = parse_stderr(Path(c["stderr_path"]))
        b_wall = float(b["wall_s"])
        c_wall = float(c["wall_s"])
        b_cpu = float(b["cpu_s"])
        c_cpu = float(c["cpu_s"])
        raw_pairs.setdefault(cell, []).append(
            {
                "pair": p,
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
                "batch_pass_wall_ms": c_info.get("batch_pass_wall_ms"),
                "batch_pass_skipped": bool(c_info.get("batch_skipped")),
                "both_clean": (
                    bool(b_info.get("ok"))
                    and bool(c_info.get("ok"))
                    and int(b["exit_code"]) == 0
                    and int(c["exit_code"]) == 0
                ),
            }
        )
    summary: dict[str, dict] = {}
    for cell in CELLS:
        plist = raw_pairs.get(cell, [])
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
        wall_saving = (1.0 - c_med / b_med) * 100.0
        cpu_saving = (1.0 - c_cpu_med / b_cpu_med) * 100.0
        pairwise = statistics.median([p["wall_saving_pct"] for p in plist])
        batch_walls = [
            p["batch_pass_wall_ms"]
            for p in plist
            if p["batch_pass_wall_ms"] is not None
        ]
        shas = {p["baseline_sha256"] for p in plist} | {
            p["candidate_sha256"] for p in plist
        }
        summary[cell] = {
            "pair_count": len(plist),
            "baseline_wall_median_s": round(b_med, 6),
            "candidate_wall_median_s": round(c_med, 6),
            "wall_delta_s": round(c_med - b_med, 6),
            "wall_saving_pct": round(wall_saving, 4),
            "wall_saving_pct_pairwise_median": round(pairwise, 4),
            "floor_multiple": round(wall_saving / FLOOR_PCT, 2),
            "clears_floor_0p80pct": wall_saving >= FLOOR_PCT,
            "baseline_cpu_median_s": round(b_cpu_med, 6),
            "candidate_cpu_median_s": round(c_cpu_med, 6),
            "cpu_saving_pct": round(cpu_saving, 4),
            "cpu_corroborates": c_cpu_med < b_cpu_med,
            "pairs_candidate_faster": sum(
                1 for p in plist if p["candidate_wall_s"] < p["baseline_wall_s"]
            ),
            "batch_pass_wall_ms_median": (
                round(statistics.median(batch_walls), 3) if batch_walls else None
            ),
            "batch_pass_skipped_all_pairs": all(p["batch_pass_skipped"] for p in plist),
            "peak_ws_baseline_median_bytes": int(
                statistics.median([p["baseline_peak_ws_bytes"] for p in plist])
            ),
            "peak_ws_candidate_median_bytes": int(
                statistics.median([p["candidate_peak_ws_bytes"] for p in plist])
            ),
            "output_sha256_stable": len(shas) == 1
            and next(iter(shas)) == ACTIVE_REFERENCE[cell],
            "all_runs_clean": all(p["both_clean"] for p in plist),
            "pairs": plist,
        }
    return summary, meta


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
        a_info = parse_stderr(Path(a["stderr_path"]))
        b_info = parse_stderr(Path(b["stderr_path"]))
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
                    bool(a_info.get("ok"))
                    and bool(b_info.get("ok"))
                    and int(a["exit_code"]) == 0
                    and int(b["exit_code"]) == 0
                ),
            }
        )
    cells_out = {}
    all_wall_abs = []
    all_cpu_abs = []
    for cell, plist in by_cell.items():
        w_max = max(abs(p["wall_delta_pct"]) for p in plist)
        c_max = max(abs(p["cpu_delta_pct"]) for p in plist)
        all_wall_abs.append(w_max)
        all_cpu_abs.append(c_max)
        cells_out[cell] = {
            "pair_count": len(plist),
            "wall_delta_pct_median": round(
                statistics.median([p["wall_delta_pct"] for p in plist]), 4
            ),
            "cpu_delta_pct_median": round(
                statistics.median([p["cpu_delta_pct"] for p in plist]), 4
            ),
            "wall_abs_max_pct": round(w_max, 4),
            "cpu_abs_max_pct": round(c_max, 4),
            "all_pairs_clean": all(p["both_clean"] for p in plist),
            "pairs": plist,
        }
    return {
        "batch_meta": meta,
        "band_pct": {
            "wall_abs_max_pct": round(max(all_wall_abs), 4) if all_wall_abs else None,
            "cpu_abs_max_pct": round(max(all_cpu_abs), 4) if all_cpu_abs else None,
        },
        "cells": cells_out,
    }


def reduce_ceiling(batch: str, ab: dict) -> dict:
    cdir = RAW / "ceiling" / batch
    if not (cdir / "rows.csv").is_file():
        return {"present": False, "cells": {}}
    rows = read_csv(cdir / "rows.csv")
    by_cell: dict[str, list[dict]] = {}
    for r in rows:
        by_cell.setdefault(r["cell"], []).append(r)
    cells = {}
    for cell, runs in by_cell.items():
        walls = [float(r["wall_s"]) for r in runs]
        cpus = [float(r["cpu_s"]) for r in runs]
        clean = all(
            bool(parse_stderr(Path(r["stderr_path"])).get("ok"))
            and int(r["exit_code"]) == 0
            for r in runs
        )
        baseline = ab.get(cell, {}).get("baseline_wall_median_s")
        null_med = statistics.median(walls)
        cells[cell] = {
            "run_count": len(runs),
            "nullrender_wall_median_s": round(null_med, 6),
            "nullrender_cpu_median_s": round(statistics.median(cpus), 6),
            "baseline_wall_median_s": baseline,
            "render_ceiling_pct": (
                round((baseline - null_med) / baseline * 100.0, 4) if baseline else None
            ),
            "all_runs_clean": clean,
        }
    return {"present": True, "cells": cells}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--hash-batch", default="hash")
    ap.add_argument("--output-batch", default="output")
    ap.add_argument("--ab-batches", nargs="+", default=["ab", "ab-off"])
    ap.add_argument("--null-batch", default="null")
    ap.add_argument("--ceiling-batch", default="ceiling")
    ap.add_argument("--out", type=Path, default=HERE / "reduced" / "campaign.json")
    args = ap.parse_args()

    manifest = json.loads((RAW / "manifest.json").read_text(encoding="utf-8-sig"))
    identity = reduce_hash(args.hash_batch)
    output = reduce_output(args.output_batch)
    ab: dict[str, dict] = {}
    ab_meta = {"batches": []}
    for batch in args.ab_batches:
        part, meta = reduce_ab(batch)
        ab.update(part)
        ab_meta["batches"].append(meta)
    null = reduce_null(args.null_batch)
    ceiling = reduce_ceiling(args.ceiling_batch, ab)

    band_wall = null["band_pct"]["wall_abs_max_pct"]
    band_cpu = null["band_pct"]["cpu_abs_max_pct"]

    verdicts = {}
    for cell in SUPPORTS_ON:
        entry = ab[cell]
        saving = entry["wall_saving_pct"]
        outside_band = (band_wall is not None) and (abs(saving) > band_wall)
        hurts = saving >= FLOOR_PCT and outside_band
        verdicts[cell] = {
            "wall_saving_pct": saving,
            "clears_floor": entry["clears_floor_0p80pct"],
            "outside_null_band": outside_band,
            "verdict": "hurts" if hurts else "exonerated",
        }

    gates = {
        "input_identity_recorded": bool(identity["cells"]),
        "input_read_set_identical_8_of_8": identity["all_cells_match"],
        "output_identical_8_of_8": all(output[c]["identical"] for c in CELLS),
        "reference_set_unmoved_8_of_8": all(
            output[c]["matches_active_reference"] for c in CELLS
        ),
        "output_runs_clean_8_of_8": all(
            output[c]["baseline_clean"] and output[c]["candidate_clean"] for c in CELLS
        ),
        "ab_output_stable_8_of_8": all(ab[c]["output_sha256_stable"] for c in CELLS),
        "ab_runs_clean_8_of_8": all(ab[c]["all_runs_clean"] for c in CELLS),
        "supports_off_batch_skipped_4_of_4": all(
            ab[c]["batch_pass_skipped_all_pairs"] for c in CELLS if c not in SUPPORTS_ON
        ),
        "null_control_complete": all(
            null["cells"].get(c, {}).get("pair_count", 0) >= 3 for c in SUPPORTS_ON
        ),
        "null_control_clean": all(
            null["cells"].get(c, {}).get("all_pairs_clean") for c in SUPPORTS_ON
        ),
    }

    payload = {
        "ticket": "t160",
        "batch": args.ab_batches,
        "manifest": manifest,
        "ab_meta": ab_meta,
        "floor_pct": FLOOR_PCT,
        "identity": identity,
        "output": output,
        "ab": ab,
        "null": null,
        "ceiling": ceiling,
        "verdicts": verdicts,
        "gates": gates,
        "all_gates_pass": all(gates.values()),
        "disclosures": [
            "One binary serves both arms; the batch arm differs only by PNP_T160_SUPPORT_BATCH=1.",
            "The batch pass clones each layer's SliceIR into a throwaway arena before dispatching; the baseline pays that clone once per layer in the ordinary path, so the batch arm carries one extra per-layer clone (conservative against the candidate).",
            "The batch arm holds every layer's SupportIR until replay; see peak_ws medians.",
            "No accelerated arm.",
            "No fresh Orca scoreboard; this is a placement experiment, not a ratio of record.",
        ],
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {args.out}")
    print(
        "gates: "
        + ", ".join(f"{k}={'PASS' if v else 'FAIL'}" for k, v in gates.items())
    )
    print("verdicts: " + ", ".join(f"{k}={v['verdict']}" for k, v in verdicts.items()))


if __name__ == "__main__":
    main()
