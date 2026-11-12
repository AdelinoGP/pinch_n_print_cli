#!/usr/bin/env python3
"""Reduce the t142 raw capture tree into the committed evidence set.

Inputs (gitignored, under ``target/t142``):
  * ``manifest.json``      arm provenance (exe / guest-module sha256)
  * ``output1/rows.csv``   one run per (cell, arm) — byte-equality check
  * ``ab/rows.csv``        interleaved paired baseline vs candidate runs

Output (committed):
  * ``reduced/campaign.json`` — every raw row plus the derived medians, ratios
    and gate results, so ``verify.py`` can re-derive the claims without the raw
    tree.

The reducer never invents a number: medians and ratios are computed here from
the CSV rows and re-computed by ``verify.py``.
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

# Reference set recorded by t139 (`93e21fee…` etc.); t142 must not move it.
T139_REFERENCE = {
    "benchy-classic-off": "93e21fee",
    "benchy-classic-on": "b3f1eccc",
    "benchy-arachne-off": "3510e876",
    "benchy-arachne-on": "96cdac85",
    "base-classic-off": "ca14d055",
    "base-classic-on": "585c6a8e",
    "base-arachne-off": "d8fdc1d9",
    "base-arachne-on": "c9fee59a",
}

# The ticket's paired median floor: a cell counts only at >= 0.80 % saved.
FLOOR_PCT = 0.80


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def median(values: list[float]) -> float:
    return statistics.median(values)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", type=Path, default=Path("target/t142"))
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(
            "docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/reduced"
        ),
    )
    args = parser.parse_args()

    manifest = json.loads((args.raw_root / "manifest.json").read_text(encoding="utf-8"))
    output_rows = read_rows(args.raw_root / "output1" / "rows.csv")
    ab_rows = read_rows(args.raw_root / "ab" / "rows.csv")
    accelerated_rows_path = args.raw_root / "output-accel2" / "rows.csv"
    accelerated_rows = (
        read_rows(accelerated_rows_path) if accelerated_rows_path.exists() else []
    )

    # ── Output gate: one run per (cell, arm); exact sha equality ────────────
    output_by_cell: dict[str, dict[str, dict[str, str]]] = {}
    for row in output_rows:
        output_by_cell.setdefault(row["cell"], {})[row["arm"]] = row
    output = {}
    for cell in CELLS:
        arms = output_by_cell[cell]
        baseline = arms["baseline-ordinary"]
        candidate = arms["candidate-ordinary"]
        identical = (
            bool(baseline["sha256"]) and baseline["sha256"] == candidate["sha256"]
        )
        output[cell] = {
            "baseline_sha256": baseline["sha256"],
            "candidate_sha256": candidate["sha256"],
            "gcode_bytes": int(candidate["gcode_bytes"]),
            "identical": identical,
            "matches_t139_reference": candidate["sha256"].startswith(
                T139_REFERENCE[cell]
            ),
        }

    # ── A/B: per-pair raw rows plus derived medians ─────────────────────────
    ab: dict[str, dict[str, dict]] = {}
    for cell in CELLS:
        cell_rows = [row for row in ab_rows if row["cell"] == cell]
        arms: dict[str, list[dict[str, str]]] = {
            "baseline-ordinary": [],
            "candidate-ordinary": [],
        }
        for row in cell_rows:
            arms[row["arm"]].append(row)
        pairs = sorted({int(row["pair"]) for row in cell_rows})
        baseline_wall = [
            float(row["wall_seconds"]) for row in arms["baseline-ordinary"]
        ]
        candidate_wall = [
            float(row["wall_seconds"]) for row in arms["candidate-ordinary"]
        ]
        baseline_cpu = [float(row["cpu_seconds"]) for row in arms["baseline-ordinary"]]
        candidate_cpu = [
            float(row["cpu_seconds"]) for row in arms["candidate-ordinary"]
        ]
        baseline_median = median(baseline_wall)
        candidate_median = median(candidate_wall)
        saving_pct = (1.0 - candidate_median / baseline_median) * 100.0
        pairs_candidate_faster = all(
            float(cand["wall_seconds"]) < float(base["wall_seconds"])
            for base, cand in zip(
                sorted(arms["baseline-ordinary"], key=lambda r: int(r["pair"])),
                sorted(arms["candidate-ordinary"], key=lambda r: int(r["pair"])),
            )
        )
        ab[cell] = {
            "pairs": pairs,
            "rows": [
                {
                    "pair": int(row["pair"]),
                    "arm": row["arm"],
                    "wall_seconds": float(row["wall_seconds"]),
                    "cpu_seconds": float(row["cpu_seconds"]),
                    "sha256": row["sha256"],
                }
                for row in sorted(cell_rows, key=lambda r: (int(r["pair"]), r["arm"]))
            ],
            "baseline_wall_median_s": round(baseline_median, 4),
            "candidate_wall_median_s": round(candidate_median, 4),
            "baseline_cpu_median_s": round(median(baseline_cpu), 4),
            "candidate_cpu_median_s": round(median(candidate_cpu), 4),
            "wall_ratio_candidate_over_baseline": round(
                candidate_median / baseline_median, 4
            ),
            "wall_saving_pct": round(saving_pct, 3),
            "cpu_ratio_candidate_over_baseline": round(
                median(candidate_cpu) / median(baseline_cpu), 4
            ),
            "cpu_corroborates": median(candidate_cpu) < median(baseline_cpu),
            "pairs_candidate_faster": pairs_candidate_faster,
            "clears_floor": saving_pct >= FLOOR_PCT,
            "output_sha256_stable": len({row["sha256"] for row in cell_rows}) == 1,
        }

    accelerated: dict[str, dict] = {}
    for row in accelerated_rows:
        cell = row["cell"]
        accelerated[cell] = {
            "sha256": row["sha256"],
            "gcode_bytes": int(row["gcode_bytes"]),
            "matches_candidate_ordinary": row["sha256"] == output[cell]["candidate_sha256"],
        }

    gates = {
        "output_identical_8_of_8": all(output[cell]["identical"] for cell in CELLS),
        "reference_set_unmoved_8_of_8": all(
            output[cell]["matches_t139_reference"] for cell in CELLS
        ),
        "timing_floor_8_of_8": all(ab[cell]["clears_floor"] for cell in CELLS),
        "cpu_corroborates_8_of_8": all(ab[cell]["cpu_corroborates"] for cell in CELLS),
        "no_cell_regresses": all(ab[cell]["wall_saving_pct"] > 0.0 for cell in CELLS),
        "pairs_candidate_faster_8_of_8": all(
            ab[cell]["pairs_candidate_faster"] for cell in CELLS
        ),
        "output_stable_across_ab_runs_8_of_8": all(
            ab[cell]["output_sha256_stable"] for cell in CELLS
        ),
    }
    if accelerated:
        gates["accelerated_output_matches_candidate_8_of_8"] = all(
            accelerated.get(cell, {}).get("matches_candidate_ordinary", False)
            for cell in CELLS
        )

    payload = {
        "ticket": 142,
        "floor_pct": FLOOR_PCT,
        "manifest": manifest,
        "output": output,
        "accelerated_output": accelerated,
        "accelerated_output_note": (
            "Manual accelerated host build (`RUSTFLAGS=--cfg "
            "pnp_perimeter_spatial_accelerated`, no controlled-driver wrapper, "
            "because the wrapper's verbatim OUT_DIR breaks libmimalloc-sys under "
            "MSVC 14.51) plus the accelerated guest artifacts from "
            "`cargo xtask build-guests --accelerated`. Output parity only; not a "
            "timing arm."
        ),
        "ab": ab,
        "gates": gates,
        "derivation_note": (
            "wall_saving_pct = (1 - candidate_median / baseline_median) * 100 over "
            "the paired uninstrumented wall rows in ab.rows; CPU is the process "
            "kernel+user total. Every value is computed from the committed rows."
        ),
    }
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "campaign.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {args.out / 'campaign.json'}")
    for name, passed in gates.items():
        print(f"  {'PASS' if passed else 'FAIL'}  {name}")
    return 0 if all(gates.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
