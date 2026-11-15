#!/usr/bin/env python3
"""Reduce the t156 raw capture tree into the committed evidence set.

Inputs (gitignored, under ``target/t156``):
  * ``manifest.json``      arm provenance (host sha256 + module-tree digest)
  * ``output/rows.csv``    16 output rows (8 cells x 2 arms)
  * ``ab/rows.csv``        48 interleaved paired rows (3 pairs x 8 cells)
  * ``null/rows.csv``      same-arm null-control rows (3 cells x 3 pairs)
  * ``output/load.json`` / ``ab/load.json`` / ``null/load.json``  box samples

Output (committed):
  * ``reduced/campaign.json`` — every raw row, each run's slice-complete
    cleanliness, the derived medians, and the gate results. ``verify.py``
    re-derives every claim from this file alone.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

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
NULL_CELLS = ["base-classic-off", "base-classic-on", "base-arachne-off"]
FLOOR_PCT = 0.80
HERE = Path(__file__).resolve().parent
REFERENCE_PATH = HERE.parent / "t151-skirt-hull-contour" / "reference.json"


def read_rows(path: Path) -> list[dict]:
    if not path.exists():
        return []
    with path.open(encoding="utf-8", newline="") as handle:
        rows = []
        for row in csv.DictReader(handle):
            if not row.get("cell"):
                continue
            row["wall_seconds"] = float(row["wall_seconds"])
            row["cpu_seconds"] = float(row["cpu_seconds"])
            row["pair"] = int(row["pair"])
            row["gcode_bytes"] = int(row["gcode_bytes"])
            row["exit_code"] = int(row["exit_code"])
            rows.append(row)
        return rows


def load_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def parse_clean(batch_dir: Path, run_id: str) -> dict:
    path = batch_dir / "gcode" / f"{run_id}.stderr.jsonl"
    clean = {
        "stderr_present": path.exists(),
        "slice_complete": None,
        "degraded": None,
        "fatal_error_count": None,
        "non_fatal_error_count": None,
        "diagnostic_lines": 0,
    }
    if not path.exists():
        return clean
    complete = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "diagnostic" in str(event.get("event", "")).lower():
            clean["diagnostic_lines"] += 1
        if event.get("event") == "slice_complete":
            complete = event
    if complete is not None:
        clean["slice_complete"] = {
            "elapsed_ms": complete.get("elapsed_ms"),
            "status": complete.get("status"),
        }
        clean["degraded"] = complete.get("degraded")
        clean["fatal_error_count"] = complete.get("fatal_error_count")
        clean["non_fatal_error_count"] = complete.get("non_fatal_error_count")
    return clean


def run_is_clean(row: dict, clean: dict) -> bool:
    return (
        row["exit_code"] == 0
        and clean["stderr_present"]
        and clean["degraded"] is False
        and clean["fatal_error_count"] == 0
        and clean["non_fatal_error_count"] == 0
        and clean["diagnostic_lines"] == 0
        and clean["slice_complete"] is not None
        and clean["slice_complete"].get("status") == "ok"
    )


def output_summary(rows: list[dict], batch_dir: Path, reference: dict) -> dict:
    cells: dict[str, dict] = {}
    for row in rows:
        run_id = f"{row['cell']}-{row['arm']}"
        clean = parse_clean(batch_dir, run_id)
        cells.setdefault(row["cell"], {})[row["arm"]] = {
            "run_id": run_id,
            "sha256": row["sha256"],
            "gcode_bytes": row["gcode_bytes"],
            "exit_code": row["exit_code"],
            "clean": clean,
        }
    output: dict[str, dict] = {}
    for cell in CELLS:
        arms = cells.get(cell, {})
        baseline = arms.get("baseline-ordinary")
        candidate = arms.get("candidate-ordinary")
        identical = bool(
            baseline and candidate and baseline["sha256"] == candidate["sha256"]
        )
        output[cell] = {
            "baseline_sha256": baseline["sha256"] if baseline else None,
            "candidate_sha256": candidate["sha256"] if candidate else None,
            "gcode_bytes": candidate["gcode_bytes"] if candidate else 0,
            "identical": identical,
            "matches_active_reference": bool(
                candidate and candidate["sha256"] == reference.get(cell)
            ),
            "baseline_clean": bool(
                baseline and run_is_clean(baseline, baseline["clean"])
            ),
            "candidate_clean": bool(
                candidate and run_is_clean(candidate, candidate["clean"])
            ),
            "baseline": baseline,
            "candidate": candidate,
        }
    return output


def ab_summary(rows: list[dict], batch_dir: Path) -> dict:
    by_cell: dict[str, dict[int, dict[str, dict]]] = {}
    for row in rows:
        if row["cell"] not in CELLS:
            continue
        run_id = f"{row['cell']}-{row['arm']}-p{row['pair']}"
        clean = parse_clean(batch_dir, run_id)
        row["run_id"] = run_id
        row["clean"] = clean
        by_cell.setdefault(row["cell"], {}).setdefault(row["pair"], {})[row["arm"]] = (
            row
        )

    out: dict[str, dict] = {}
    for cell in CELLS:
        pairs = by_cell.get(cell, {})
        records = []
        for pair in sorted(pairs):
            arms = pairs[pair]
            base = arms.get("baseline-ordinary")
            cand = arms.get("candidate-ordinary")
            if base is None or cand is None:
                continue
            records.append(
                {
                    "pair": pair,
                    "order": base.get("order", ""),
                    "baseline_wall_s": base["wall_seconds"],
                    "candidate_wall_s": cand["wall_seconds"],
                    "baseline_cpu_s": base["cpu_seconds"],
                    "candidate_cpu_s": cand["cpu_seconds"],
                    "baseline_sha256": base["sha256"],
                    "candidate_sha256": cand["sha256"],
                    "wall_delta_pct": round(
                        (cand["wall_seconds"] - base["wall_seconds"])
                        / base["wall_seconds"]
                        * 100.0,
                        4,
                    ),
                    "cpu_delta_pct": round(
                        (cand["cpu_seconds"] - base["cpu_seconds"])
                        / base["cpu_seconds"]
                        * 100.0,
                        4,
                    ),
                    "baseline_clean": {
                        "ok": run_is_clean(base, base["clean"]),
                        **base["clean"],
                    },
                    "candidate_clean": {
                        "ok": run_is_clean(cand, cand["clean"]),
                        **cand["clean"],
                    },
                }
            )
        if not records:
            continue
        baseline_wall = statistics.median(r["baseline_wall_s"] for r in records)
        candidate_wall = statistics.median(r["candidate_wall_s"] for r in records)
        baseline_cpu = statistics.median(r["baseline_cpu_s"] for r in records)
        candidate_cpu = statistics.median(r["candidate_cpu_s"] for r in records)
        saving_pct = (1.0 - candidate_wall / baseline_wall) * 100.0
        out[cell] = {
            "pairs": records,
            "pair_count": len(records),
            "baseline_wall_median_s": round(baseline_wall, 4),
            "candidate_wall_median_s": round(candidate_wall, 4),
            "baseline_cpu_median_s": round(baseline_cpu, 4),
            "candidate_cpu_median_s": round(candidate_cpu, 4),
            "wall_saving_pct": round(saving_pct, 3),
            "wall_ratio_candidate_over_baseline": round(
                candidate_wall / baseline_wall, 4
            ),
            "cpu_ratio_candidate_over_baseline": round(candidate_cpu / baseline_cpu, 4),
            "cpu_corroborates": candidate_cpu < baseline_cpu,
            "pairs_candidate_faster": sum(
                1
                for record in records
                if record["candidate_wall_s"] < record["baseline_wall_s"]
            ),
            "clears_floor": saving_pct >= FLOOR_PCT,
            "no_cell_regression": saving_pct > 0.0,
            "output_sha256_stable": len(
                {record["baseline_sha256"] for record in records}
                | {record["candidate_sha256"] for record in records}
            )
            == 1,
            "all_runs_clean": all(
                record["baseline_clean"]["ok"] and record["candidate_clean"]["ok"]
                for record in records
            ),
        }
    return out


def null_summary(rows: list[dict], batch_dir: Path) -> dict:
    grouped: dict[tuple[str, int], list[dict]] = {}
    for row in rows:
        if row["cell"] not in NULL_CELLS:
            continue
        grouped.setdefault((row["cell"], row["pair"]), []).append(row)
    per_cell: dict[str, list[dict]] = {}
    for (cell, pair), entries in sorted(grouped.items()):
        if len(entries) != 2:
            continue
        for position, entry in enumerate(entries, start=1):
            entry["run_id"] = f"{cell}-{entry['arm']}-p{pair}-r{position}"
            entry["clean"] = parse_clean(batch_dir, entry["run_id"])
        first, second = entries
        per_cell.setdefault(cell, []).append(
            {
                "pair": pair,
                "order": first.get("order", ""),
                "first_wall_s": first["wall_seconds"],
                "second_wall_s": second["wall_seconds"],
                "first_cpu_s": first["cpu_seconds"],
                "second_cpu_s": second["cpu_seconds"],
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
                "first_sha256": first["sha256"],
                "second_sha256": second["sha256"],
                "both_clean": bool(
                    run_is_clean(first, first["clean"])
                    and run_is_clean(second, second["clean"])
                ),
            }
        )
    cells: dict[str, dict] = {}
    for cell, records in per_cell.items():
        cells[cell] = {
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
            "all_pairs_clean": all(r["both_clean"] for r in records),
        }
    band = {
        "wall_abs_max_pct": round(
            max((c["wall_delta_pct_abs_max"] for c in cells.values()), default=0.0),
            4,
        ),
        "cpu_abs_max_pct": round(
            max((c["cpu_delta_pct_abs_max"] for c in cells.values()), default=0.0),
            4,
        ),
    }
    return {"cells": cells, "band_pct": band}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", type=Path, default=Path("target/t156"))
    parser.add_argument("--out", type=Path, default=HERE / "reduced" / "campaign.json")
    args = parser.parse_args()

    reference_doc = load_json(REFERENCE_PATH) or {}
    reference = reference_doc.get("active_set", {})
    manifest = load_json(args.raw_root / "manifest.json")
    if manifest is None:
        raise SystemExit(f"missing manifest: {args.raw_root / 'manifest.json'}")

    output_batch = args.raw_root / "output"
    ab_batch = args.raw_root / "ab"
    null_batch = args.raw_root / "null"
    output = output_summary(
        read_rows(output_batch / "rows.csv"), output_batch, reference
    )
    ab = ab_summary(read_rows(ab_batch / "rows.csv"), ab_batch)
    null = null_summary(read_rows(null_batch / "rows.csv"), null_batch)

    drop_cells = [cell for cell in CELLS if not ab[cell]["clears_floor"]]
    gates = {
        "output_identical_8_of_8": all(output[cell]["identical"] for cell in CELLS),
        "reference_set_unmoved_8_of_8": all(
            output[cell]["matches_active_reference"] for cell in CELLS
        ),
        "output_runs_clean_8_of_8": all(
            output[cell]["baseline_clean"] and output[cell]["candidate_clean"]
            for cell in CELLS
        ),
        "timing_floor_8_of_8": not drop_cells,
        "cpu_corroborates_8_of_8": all(ab[cell]["cpu_corroborates"] for cell in CELLS),
        "no_cell_regresses": all(ab[cell]["no_cell_regression"] for cell in CELLS),
        "ab_output_stable_8_of_8": all(
            ab[cell]["output_sha256_stable"] for cell in CELLS
        ),
        "ab_runs_clean_8_of_8": all(ab[cell]["all_runs_clean"] for cell in CELLS),
        "null_control_complete": set(null["cells"]) == set(NULL_CELLS)
        and all(entry["pair_count"] == 3 for entry in null["cells"].values()),
        "null_control_clean": all(
            entry["all_pairs_clean"] for entry in null["cells"].values()
        ),
    }
    verdict = "KEEP" if all(gates.values()) else "DROP"

    payload = {
        "ticket": 156,
        "verdict": verdict,
        "drop_cells": drop_cells,
        "floor_pct": FLOOR_PCT,
        "reference_source": str(
            REFERENCE_PATH.relative_to(HERE.parent.parent.parent.parent)
        ),
        "reference_note": reference_doc.get("note", ""),
        "active_reference": reference,
        "manifest": manifest,
        "load": {
            "output": load_json(output_batch / "load.json"),
            "ab": load_json(ab_batch / "load.json"),
            "ab_loaded_window": {
                "first_pass": load_json(
                    args.raw_root / "ab-loaded" / "load-first-pass.json"
                ),
                "extension": load_json(args.raw_root / "ab-loaded" / "load.json"),
            },
            "null": load_json(null_batch / "load.json"),
        },
        "output": output,
        "ab": ab,
        "null": null,
        "gates": gates,
        "derivation_note": (
            "wall_saving_pct = (1 - candidate_median / baseline_median) * 100 "
            "over the paired uninstrumented wall rows; CPU is the process "
            "kernel+user total. Cleanliness is the slice_complete event's "
            "degraded/fatal/non-fatal fields plus zero diagnostic events. Every "
            "value is recomputed by verify.py from this payload."
        ),
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {args.out}  verdict={verdict}  drop_cells={drop_cells}")
    for name, passed in gates.items():
        print(f"  {'PASS' if passed else 'FAIL'}  {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
