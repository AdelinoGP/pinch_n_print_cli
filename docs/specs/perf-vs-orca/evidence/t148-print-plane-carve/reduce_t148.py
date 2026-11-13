#!/usr/bin/env python3
"""Reduce the t148 raw tree into `census.json`.

Raw inputs (all gitignored):

    target/t148/{benchy,base}-{classic,arachne}-{on,off}.gcode
    target/t148/*.stderr.log
    target/matched-pair/t148-plane-carve/scoreboard-rows.csv
    target/matched-pair/t145-benchy/gcode/*-pnp-ordinary-m3.gcode
    target/t147/probe-{benchy3,base3}/candidates.json

In-repo inputs:

    docs/specs/perf-vs-orca/evidence/t147-supports-rejection/census.json
    docs/specs/perf-vs-orca/evidence/t144-brim-anchor/reference.json

The support-path/coverage parser is imported from the t147 reducer on purpose:
t147 measured the missing support with it, so the recovery is quoted on the same
metric. The reducer needs `shapely`; `verify.py` is stdlib-only and reads only
`census.json`.
"""

from __future__ import annotations

import csv
import hashlib
import json
import statistics
import sys
from pathlib import Path

try:
    from shapely.geometry import LineString
    from shapely.ops import unary_union
except ImportError:  # pragma: no cover - documented prerequisite
    print("reduce_t148.py needs shapely (pip install shapely)", file=sys.stderr)
    raise

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).resolve().parent
sys.path.insert(
    0, str(ROOT / "docs/specs/perf-vs-orca/evidence/t147-supports-rejection")
)

from reduce_t147 import (  # noqa: E402
    LAYER_HEIGHT_MM,
    coverage,
    parse_gcode,
    parse_support_geometry,
    polygon_of,
    support_segments_at,
)

SUPPORT_HALF_WIDTH_MM = 0.2
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
# t147's retained-layer control sample: candidates on layers that were never
# dropped. Same metric, same planner, so the coverage reading for the recovered
# layers can be compared against ordinary tree-interface geometry.
RETAINED_CONTROL_CANDIDATES = [5, 10, 11, 12, 13, 14, 16, 20, 30]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def raw_gcode(cell: str) -> Path:
    return ROOT / "target/t148" / f"{cell}.gcode"


def stderr_path(cell: str) -> Path:
    return ROOT / "target/t148" / f"{cell}.stderr.log"


def diagnostic_count(path: Path) -> int:
    if not path.exists():
        return -1
    return sum(
        1
        for line in path.read_text(encoding="utf-8", errors="ignore").splitlines()
        if "exact-Z occupancy" in line
    )


def support_census(path: Path) -> dict:
    layers = parse_gcode(path)
    per_layer = {
        z: round(sum(bucket.values()), 3)
        for z, bucket in layers.items()
        if sum(bucket.values()) > 0
    }
    return {
        "support_path_mm": round(sum(per_layer.values()), 3),
        "support_layers": len(per_layer),
        "support_layer_z": sorted(per_layer),
        "per_layer_mm": {f"{z:.4f}": value for z, value in sorted(per_layer.items())},
    }


def medians(fixture: str, generator: str) -> dict:
    rows_path = ROOT / "target/matched-pair/t148-plane-carve/scoreboard-rows.csv"
    measured = {"orca": [], "pnp-ordinary": []}
    with rows_path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row["warmup"] == "1":
                continue
            if row["fixture"] != fixture or row["generator"] != generator:
                continue
            if row["tool"] in measured:
                measured[row["tool"]].append(
                    {
                        "wall_s": float(row["wall_seconds"]),
                        "cpu_s": float(row["cpu_seconds"]),
                    }
                )
    out = {}
    for tool, samples in measured.items():
        walls = [sample["wall_s"] for sample in samples]
        cpus = [sample["cpu_s"] for sample in samples]
        out[tool] = {
            "runs": len(samples),
            "wall_median_s": round(statistics.median(walls), 3),
            "cpu_median_s": round(statistics.median(cpus), 3),
            "wall_samples_s": walls,
        }
    out["ratio"] = round(
        out["pnp-ordinary"]["wall_median_s"] / out["orca"]["wall_median_s"], 3
    )
    return out


def candidate_coverage(
    pnp_geometry: dict,
    candidates: dict,
    candidate_id: int,
    plane_mm: float,
) -> dict:
    candidate = candidates.get(candidate_id)
    if candidate is None:
        return {"candidate_id": candidate_id, "known": False}
    shapes = [polygon_of(item) for item in candidate["geometry"]]
    planes = sorted(z for z in pnp_geometry if z is not None)
    below = [
        plane
        for plane in planes
        if plane <= plane_mm + 0.02 and plane_mm - plane <= 5.0
    ]
    plane = max(below) if below else None
    segments = support_segments_at(pnp_geometry, plane) if plane is not None else []
    covers = [coverage(shape, segments) for shape in shapes]
    return {
        "candidate_id": candidate_id,
        "known": True,
        "candidate_layer": candidate["layer"],
        "area_mm2": round(sum(shape.area for shape in shapes), 3),
        "plane_mm": plane,
        "pnp_coverage": round(max(covers) if covers else 0.0, 4),
    }


def main() -> int:
    t147 = json.loads(
        (
            ROOT
            / "docs/specs/perf-vs-orca/evidence/t147-supports-rejection/census.json"
        ).read_text()
    )["cells"]
    t144 = json.loads(
        (
            ROOT / "docs/specs/perf-vs-orca/evidence/t144-brim-anchor/reference.json"
        ).read_text()
    )["candidate_set"]

    cells = {}
    coverage_by_cell = {}
    for cell in SUPPORTS_ON:
        before_path = (
            ROOT
            / "target/matched-pair/t145-benchy/gcode"
            / f"{cell}-pnp-ordinary-m3.gcode"
        )
        after_path = raw_gcode(cell)
        before = support_census(before_path)
        after = support_census(after_path)
        old_z = set(before["support_layer_z"])
        new_z = set(after["support_layer_z"])
        fixture = cell.split("-")[0]
        # t147 probed one classic supports-on cell per fixture; the rejection
        # census is identical on the Arachne sibling (same 101 / 44,474
        # diagnostics, same layer set), so the classic census is the shape and
        # each cell's own G-code supplies the paths.
        probe_cell = f"{fixture}-classic-on"
        rejected = t147[probe_cell]
        candidates_path = ROOT / f"target/t147/probe-{fixture}3/candidates.json"
        candidates = {
            item["id"]: item
            for item in json.loads(candidates_path.read_text())["candidates"]
        }
        pnp_geometry = parse_support_geometry(after_path)

        recovered = []
        for entry in rejected["entries"]:
            z = round(entry["z_mm"], 4)
            recovered.append(
                {
                    "layer": entry["layer"],
                    "z_mm": z,
                    "path_mm": after["per_layer_mm"].get(f"{z:.4f}", 0.0),
                    "t147_estimate_mm": entry["missing_path_estimate_mm"],
                }
            )
        recovered.sort(key=lambda item: item["layer"])

        # Coverage: every candidate that t147 recorded as immediately above a
        # rejected layer. The plane is the one the validator uses (layer.z).
        coverage_rows = []
        for entry in rejected["entries"]:
            for item in entry["immediate_candidates"]:
                row = candidate_coverage(
                    pnp_geometry, candidates, item["candidate_id"], item["pnp_plane_mm"]
                )
                row["layer"] = entry["layer"]
                row["orca_coverage"] = item["orca_coverage"]
                row["pnp_coverage_before"] = item["pnp_coverage"]
                coverage_rows.append(row)
        coverage_by_cell[cell] = coverage_rows

        delta_path = round(after["support_path_mm"] - before["support_path_mm"], 3)
        cells[cell] = {
            "fixture": fixture,
            "before": {
                "gcode_sha256": sha256(before_path),
                **before,
                "diagnostics": t147[probe_cell]["diagnostics"],
                "degraded": True,
            },
            "after": {
                "gcode_sha256": sha256(after_path),
                **after,
                "gcode_bytes": after_path.stat().st_size,
                "diagnostics": diagnostic_count(stderr_path(cell)),
                "degraded": False,
            },
            "delta_support_path_mm": delta_path,
            "delta_support_layers": after["support_layers"] - before["support_layers"],
            "recovered_layers": recovered,
            "recovered_z": sorted(new_z - old_z),
            "lost_z": sorted(old_z - new_z),
            # t147 recorded the support path it expected the missing layers to
            # carry, by neighbour interpolation across each gap.
            "t147_missing_path_mm": rejected["missing_support_path_mm"],
            "t147_missing_share_pct": rejected["missing_share_pct"],
            "wall": medians(fixture, cell.split("-")[1]),
            "t145_wall_ratio": {
                "benchy-classic-on": 2.683,
                "benchy-arachne-on": 2.580,
                "base-classic-on": 3.001,
                "base-arachne-on": 3.455,
            }[cell],
        }

    # Retained-layer control: ordinary tree-interface coverage on layers the
    # gate never dropped, measured on the same post-fix G-code.
    retained_control = {}
    for fixture, cell in (("benchy", "benchy-classic-on"), ("base", "base-classic-on")):
        candidates = {
            item["id"]: item
            for item in json.loads(
                (ROOT / f"target/t147/probe-{fixture}3/candidates.json").read_text()
            )["candidates"]
        }
        pnp_geometry = parse_support_geometry(raw_gcode(cell))
        rows = []
        for candidate_id in RETAINED_CONTROL_CANDIDATES:
            candidate = candidates.get(candidate_id)
            if candidate is None:
                continue
            plane = round(candidate["layer"] * LAYER_HEIGHT_MM, 4)
            row = candidate_coverage(pnp_geometry, candidates, candidate_id, plane)
            rows.append(row)
        retained_control[cell] = rows

    off = {}
    for cell in SUPPORTS_OFF:
        path = raw_gcode(cell)
        digest = sha256(path)
        off[cell] = {
            "gcode_sha256": digest,
            "active_sha256": t144[cell],
            "matches_active": digest == t144[cell],
            "diagnostics": diagnostic_count(stderr_path(cell)),
            "support_path_mm": support_census(path)["support_path_mm"],
        }

    census = {
        "schema": "t148-print-plane-carve/v1",
        "note": (
            "Supports-on before/after for the four cells the exact-Z body "
            "rejection dropped support on, the four supports-off identity "
            "checks, the recovered-layer census on the t147 metric, the "
            "immediate-candidate coverage reading, and the matched-pair wall "
            "medians for the four supports-on cells."
        ),
        "cells": cells,
        "supports_off": off,
        "coverage": coverage_by_cell,
        "retained_layer_control": retained_control,
        "metric": {
            "support_path": (
                "t147 parser: extruding G1/G0 distance under a Support* "
                "feature, Support + Support interface"
            ),
            "coverage": (
                "fraction of the candidate overhang polygon covered by support "
                "paths buffered by 0.2 mm (t147's metric)"
            ),
            "wall": "median of 3 measured uninstrumented runs, 1 warmup, RAYON_NUM_THREADS=12",
        },
        "disclosures": [
            "The four supports-off cells are byte-identical to the t144 active "
            "set and were not re-recorded; only the four supports-on cells moved.",
            "No accelerated timing arm: the accelerated host remains unbuildable "
            "in libmimalloc-sys (t142/t145 disclosure).",
            "The wall window is a different window from t145; Orca's own medians "
            "moved between the two batches, so no causal wall cost is claimed "
            "from the cross-take deltas.",
            "Candidate coverage is reported, not used as a pass gate: PnP tree "
            "interfaces do not cover whole overhang polygons even on layers the "
            "gate never dropped (see retained_layer_control).",
        ],
    }
    (OUT / "census.json").write_text(
        json.dumps(census, indent=2) + "\n", encoding="utf-8"
    )

    print(f"wrote {OUT / 'census.json'}")
    for cell, data in cells.items():
        print(
            f"{cell:22s} path {data['before']['support_path_mm']:12.3f} -> "
            f"{data['after']['support_path_mm']:12.3f} "
            f"({data['delta_support_path_mm']:+.3f}) "
            f"layers {data['before']['support_layers']} -> {data['after']['support_layers']} "
            f"recovered {len(data['recovered_z'])} lost {len(data['lost_z'])} "
            f"diag {data['before']['diagnostics']} -> {data['after']['diagnostics']} "
            f"ratio {data['wall']['ratio']}"
        )
    for cell, data in off.items():
        print(
            f"{cell:22s} identity={data['matches_active']} diag={data['diagnostics']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
