#!/usr/bin/env python3
"""t164 phase re-attribution -- reduce the paired scoreboard to `reduced/board.json`.

Inputs (gitignored raw, reproducible from REPRODUCE.md):
  target/matched-pair/t164-phase-split/scoreboard-rows.csv
  target/t164-load.csv

Every PnP run's G-code SHA-256 is checked against t151's active reference set
(the matched job's currency). The board records, per cell, the three measured
runs of each tool, their medians, the spread, the ratio and the required cut.

Usage:
  python reduce_scoreboard.py [--csv ...] [--load ...] [--out reduced/board.json]
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
REFERENCE = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
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
MATCHED_KEYS = [
    "resolved_brim_width",
    "resolved_brim_type",
    "resolved_skirt_loops",
    "resolved_skirt_distance",
    "resolved_skirt_height",
    "resolved_initial_layer_line_width",
    "resolved_tree_support_brim_width",
    "resolved_enable_arc_fitting",
    "resolved_elefant_foot_compensation",
]
EXPECTED_KEYS = {
    "resolved_brim_width": "8",
    "resolved_brim_type": "outer_only",
    "resolved_skirt_loops": "6",
    "resolved_skirt_distance": "3",
    "resolved_skirt_height": "1",
    "resolved_initial_layer_line_width": "0.4",
    "resolved_tree_support_brim_width": "0",
    "resolved_enable_arc_fitting": "disabled",
    "resolved_elefant_foot_compensation": "0",
}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def spread_pct(values: list[float]) -> float:
    if len(values) < 2:
        return 0.0
    med = statistics.median(values)
    return round(100.0 * (max(values) - min(values)) / med, 2) if med else 0.0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--csv",
        default=str(
            REPO
            / "target"
            / "matched-pair"
            / "t164-phase-split"
            / "scoreboard-rows.csv"
        ),
    )
    ap.add_argument("--load", default=str(REPO / "target" / "t164-load.csv"))
    ap.add_argument("--out", default=str(HERE / "reduced" / "board.json"))
    a = ap.parse_args()

    refs = json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]
    rows = list(csv.DictReader(Path(a.csv).open(encoding="utf-8-sig")))
    measured = [r for r in rows if r["warmup"] == "0"]
    warmups = [r for r in rows if r["warmup"] == "1"]

    cells: dict[str, dict] = {}
    for cell in CELLS:
        fixture, generator, supports = cell.split("-", 2)
        # The rig's `cell` column is generator-supports only; the fixture is its
        # own column, so both are needed to isolate one matched cell.
        cell_rows = [
            r
            for r in measured
            if r["cell"] == f"{generator}-{supports}" and r["fixture"] == fixture
        ]
        orca = [r for r in cell_rows if r["tool"] == "orca"]
        pnp = [r for r in cell_rows if r["tool"] == "pnp-ordinary"]
        if len(orca) != 3 or len(pnp) != 3:
            raise SystemExit(
                f"{cell}: expected 3 measured runs per tool, got orca={len(orca)} pnp={len(pnp)}"
            )

        def series(group: list[dict], field: str) -> list[float]:
            return sorted(float(r[field]) for r in group)

        orca_wall = series(orca, "wall_seconds")
        orca_cpu = series(orca, "cpu_seconds")
        pnp_wall = series(pnp, "wall_seconds")
        pnp_cpu = series(pnp, "cpu_seconds")
        pnp_hashes = [r["sha256_of_gcode"] for r in pnp]
        identity = all(h == refs[cell] for h in pnp_hashes)
        ratio = statistics.median(pnp_wall) / statistics.median(orca_wall)
        cells[cell] = {
            "fixture": fixture,
            "generator": generator,
            "supports": supports,
            "orca": {
                "runs": len(orca),
                "wall_s": orca_wall,
                "cpu_s": orca_cpu,
                "wall_median_s": round(statistics.median(orca_wall), 4),
                "cpu_median_s": round(statistics.median(orca_cpu), 4),
                "wall_spread_pct": spread_pct(orca_wall),
                "cpu_spread_pct": spread_pct(orca_cpu),
            },
            "pnp_ordinary": {
                "runs": len(pnp),
                "wall_s": pnp_wall,
                "cpu_s": pnp_cpu,
                "wall_median_s": round(statistics.median(pnp_wall), 4),
                "cpu_median_s": round(statistics.median(pnp_cpu), 4),
                "wall_spread_pct": spread_pct(pnp_wall),
                "cpu_spread_pct": spread_pct(pnp_cpu),
            },
            "ratio_ordinary_over_orca": round(ratio, 4),
            "required_cut_ordinary_pct": round(100.0 * (1.0 - 1.0 / ratio), 2),
            "identity": identity,
            "hash_reference": refs[cell],
            "hash_runs": pnp_hashes,
            "exit_codes": sorted({r["exit_code"] for r in cell_rows}),
            "completion_status_pnp": sorted({r["completion_status"] for r in pnp}),
            "degraded_pnp": sorted({r["degraded"] for r in pnp}),
            "fatal_error_count_pnp": sorted({r["fatal_error_count"] for r in pnp}),
            "non_fatal_error_count_pnp": sorted(
                {r["non_fatal_error_count"] for r in pnp}
            ),
            "matched_keys": {k: sorted({r[k] for r in orca}) for k in MATCHED_KEYS},
            "matched_keys_ok": all(
                sorted({r[k] for r in orca}) == [EXPECTED_KEYS[k]] for k in MATCHED_KEYS
            ),
        }

    load_values: list[float] = []
    load_path = Path(a.load)
    if load_path.exists():
        for line in load_path.read_text(encoding="utf-8").splitlines():
            parts = line.strip().split(",")
            if len(parts) == 2:
                try:
                    v = float(parts[1])
                except ValueError:
                    continue
                if v >= 0:
                    load_values.append(v)
        (HERE / "reduced").mkdir(parents=True, exist_ok=True)
        (HERE / "reduced" / "load-samples.csv").write_text(
            load_path.read_text(encoding="utf-8"), encoding="utf-8"
        )

    payload = {
        "ticket": "164",
        "generated_by": "reduce_scoreboard.py",
        "protocol": {
            "cells": len(CELLS),
            "tools": ["orca", "pnp-ordinary"],
            "runs_per_cell_tool": 3,
            "warmups_per_cell_tool": 1,
            "interleaved": True,
            "measured_rows": len(measured),
            "warmup_rows": len(warmups),
            "threads": sorted({r["threads"] for r in rows}),
        },
        "tool_identity": {
            "orca_exe_path": sorted(
                {r["exe_path"] for r in rows if r["tool"] == "orca"}
            )[0],
            "orca_exe_sha256": sha256(
                Path("C:/Program Files/OrcaSlicer/orca-slicer.exe")
            )
            if Path("C:/Program Files/OrcaSlicer/orca-slicer.exe").exists()
            else "",
            "pnp_exe_path": sorted(
                {r["exe_path"] for r in rows if r["tool"] == "pnp-ordinary"}
            )[0],
            "pnp_exe_sha256": sha256(REPO / "target" / "release" / "pnp_cli.exe"),
        },
        "currency_artifact": "docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json",
        "box_load_pct": {
            "samples": len(load_values),
            "min": round(min(load_values), 1) if load_values else None,
            "median": round(statistics.median(load_values), 1) if load_values else None,
            "max": round(max(load_values), 1) if load_values else None,
        },
        "cells": cells,
    }
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    wins = sum(1 for c in cells.values() if c["ratio_ordinary_over_orca"] <= 1.0)
    ident = sum(1 for c in cells.values() if c["identity"])
    print(f"wrote {a.out}: {len(cells)} cells, identity {ident}/8, win {wins}/8")
    for cell in CELLS:
        c = cells[cell]
        print(
            f"  {cell:20s} orca {c['orca']['wall_median_s']:8.3f}s  "
            f"pnp {c['pnp_ordinary']['wall_median_s']:8.3f}s  "
            f"{c['ratio_ordinary_over_orca']:5.3f}x  cut {c['required_cut_ordinary_pct']:5.2f}%"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
