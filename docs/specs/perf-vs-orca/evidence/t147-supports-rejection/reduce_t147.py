#!/usr/bin/env python3
"""Reduce the t147 raw probe tree into `census.json`.

Raw inputs (all gitignored, produced by the probe described in REPRODUCE.md):

    target/t147/probe-benchy3/entries.jsonl
    target/t147/probe-benchy3/candidates.json
    target/t147/probe-benchy3/stderr.log
    target/t147/probe-base3/entries.jsonl
    target/t147/probe-base3/candidates.json
    target/t147/probe-base3/stderr.log
    target/t147/benchy-classic-on.probe3.gcode
    target/t147/base-classic-on.probe3.gcode
    target/matched-pair/t145-benchy/gcode/benchy-classic-on-orca-m3.gcode
    target/matched-pair/t145-benchy/gcode/base-classic-on-orca-m3.gcode
    docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json

The reducer needs `shapely` (same prerequisite as t145's cross-section reducer).
`verify.py` is stdlib-only and reads only `census.json`.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

try:
    from shapely import make_valid
    from shapely.geometry import LineString, Polygon
    from shapely.ops import unary_union
except ImportError:  # pragma: no cover - documented prerequisite
    print("reduce_t147.py needs shapely (pip install shapely)", file=sys.stderr)
    raise

ROOT = Path(__file__).resolve().parents[5]
OUT = Path(__file__).resolve().parent
LAYER_HEIGHT_MM = 0.2
SUPPORT_HALF_WIDTH_MM = 0.2
ORCA_LOOKBACK_MM = 0.35


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_gcode(path: Path) -> dict[float, dict[str, float]]:
    """Per-layer extrusion path length, split by Support / Support interface.

    The parser is deliberately the same one applied to both arms, so the PnP
    and Orca support-path totals are comparable even though they are not the
    `support path` metric used by t145's G-code reducer.
    """
    layers: dict[float, dict[str, float]] = {}
    z = None
    feature = None
    x = y = None
    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            text = line.strip()
            if text.startswith(";"):
                match = re.match(r"; ?Z(?:_HEIGHT)?:\s*([0-9.]+)", text)
                if match:
                    z = round(float(match.group(1)), 4)
                    x = y = None
                    continue
                match = re.match(r"; ?FEATURE:\s*(.+)", text) or re.match(
                    r";TYPE:\s*(.+)", text
                )
                if match:
                    feature = match.group(1).strip()
                continue
            if not (text.startswith("G1 ") or text.startswith("G0 ")):
                continue
            mx = re.search(r"X([-0-9.]+)", text)
            my = re.search(r"Y([-0-9.]+)", text)
            nx = float(mx.group(1)) if mx else x
            ny = float(my.group(1)) if my else y
            me = re.search(r"\bE(-?[0-9.]+)", text)
            extruding = bool(me) and float(me.group(1)) > 0
            if (
                feature
                and feature.startswith("Support")
                and extruding
                and x is not None
                and y is not None
            ):
                bucket = layers.setdefault(
                    z, {"Support": 0.0, "Support interface": 0.0}
                )
                bucket[feature if feature in bucket else "Support"] += math.hypot(
                    nx - x, ny - y
                )
            x, y = nx, ny
    return layers


def support_segments_at(
    layers: dict[float, dict[str, list]], z: float, band: float = 0.002
) -> list[tuple[tuple[float, float], tuple[float, float]]]:
    """Support segments at one exact plane (the path parser discards them)."""
    segments = []
    for zz, payload in layers.items():
        if zz is not None and abs(zz - z) <= band:
            for feature, lines in payload.items():
                if feature.startswith("Support"):
                    segments.extend(lines)
    return segments


def parse_support_geometry(path: Path) -> dict[float, dict[str, list]]:
    layers: dict[float, dict[str, list]] = {}
    z = None
    feature = None
    x = y = None
    with path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            text = line.strip()
            if text.startswith(";"):
                match = re.match(r"; ?Z(?:_HEIGHT)?:\s*([0-9.]+)", text)
                if match:
                    z = round(float(match.group(1)), 4)
                    x = y = None
                    continue
                match = re.match(r"; ?FEATURE:\s*(.+)", text) or re.match(
                    r";TYPE:\s*(.+)", text
                )
                if match:
                    feature = match.group(1).strip()
                continue
            if not (text.startswith("G1 ") or text.startswith("G0 ")):
                continue
            mx = re.search(r"X([-0-9.]+)", text)
            my = re.search(r"Y([-0-9.]+)", text)
            nx = float(mx.group(1)) if mx else x
            ny = float(my.group(1)) if my else y
            me = re.search(r"\bE(-?[0-9.]+)", text)
            extruding = bool(me) and float(me.group(1)) > 0
            if (
                feature
                and feature.startswith("Support")
                and extruding
                and x is not None
                and y is not None
            ):
                layers.setdefault(z, {}).setdefault(feature, []).append(
                    ((x, y), (nx, ny))
                )
            x, y = nx, ny
    return layers


def polygon_of(geometry: dict) -> Polygon:
    shell = [(x / 1e4, y / 1e4) for x, y in geometry["contour"]]
    holes = [[(x / 1e4, y / 1e4) for x, y in hole] for hole in geometry["holes"]]
    shape = Polygon(shell, holes)
    return shape if shape.is_valid else make_valid(shape)


def coverage(
    polygon: Polygon, segments: list, half: float = SUPPORT_HALF_WIDTH_MM
) -> float:
    if not segments or polygon is None or polygon.is_empty or polygon.area <= 0:
        return 0.0
    lines = unary_union([LineString(segment) for segment in segments])
    return polygon.intersection(lines.buffer(half, cap_style=2)).area / polygon.area


def load_entries(path: Path) -> list[dict]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def demand_candidate(demand: str) -> int | None:
    match = re.match(r"(?:mesh-)?demand-(\d+)", demand)
    return int(match.group(1)) if match else None


def census_cell(
    label: str,
    entries_path: Path,
    stderr_path: Path,
    candidates_path: Path | None,
    pnp_gcode: Path,
    orca_gcode: Path,
    window: tuple[int, int] | None,
) -> dict:
    rows = load_entries(entries_path)
    rejected = [row for row in rows if row["verdict"]]
    retained = [row for row in rows if not row["verdict"]]
    diagnostics = 0
    with stderr_path.open("r", encoding="utf-8", errors="ignore") as handle:
        for line in handle:
            if "exact-Z occupancy" in line:
                diagnostics += 1

    rejected_demands = {demand for row in rejected for demand in row["demand_ids"]}
    retained_demands = {demand for row in retained for demand in row["demand_ids"]}
    only_rejected = rejected_demands - retained_demands

    pnp_path = parse_gcode(pnp_gcode)
    orca_path = parse_gcode(orca_gcode)
    pnp_total = sum(sum(bucket.values()) for bucket in pnp_path.values())
    orca_total = sum(sum(bucket.values()) for bucket in orca_path.values())

    orca_geometry = parse_support_geometry(orca_gcode)
    orca_planes = sorted(z for z in orca_geometry if z is not None)
    candidates = {}
    if candidates_path is not None and candidates_path.exists():
        with candidates_path.open("r", encoding="utf-8") as handle:
            payload = json.load(handle)
        candidates = {item["id"]: item for item in payload["candidates"]}

    def z_of(layer: int) -> float:
        return round(layer * LAYER_HEIGHT_MM + LAYER_HEIGHT_MM, 3)

    def neighbor_estimate(layer: int) -> dict:
        z = z_of(layer)
        present = pnp_path.get(z, {}).get("Support", 0.0) + pnp_path.get(z, {}).get(
            "Support interface", 0.0
        )
        if present > 0:
            return {
                "layer": layer,
                "z_mm": z,
                "present_mm": present,
                "estimate_mm": 0.0,
            }
        below = []
        above = []
        for step in range(1, 6):
            zz = z_of(layer - step)
            value = pnp_path.get(zz, {}).get("Support", 0.0) + pnp_path.get(zz, {}).get(
                "Support interface", 0.0
            )
            if value > 0:
                below.append({"layer": layer - step, "z_mm": zz, "mm": round(value, 3)})
                break
        for step in range(1, 7):
            zz = z_of(layer + step)
            value = pnp_path.get(zz, {}).get("Support", 0.0) + pnp_path.get(zz, {}).get(
                "Support interface", 0.0
            )
            if value > 0:
                above.append({"layer": layer + step, "z_mm": zz, "mm": round(value, 3)})
                break
        values = [item["mm"] for item in below[:1] + above[:1]]
        estimate = sum(values) / len(values) if values else 0.0
        return {
            "layer": layer,
            "z_mm": z,
            "present_mm": 0.0,
            "estimate_mm": round(estimate, 3),
            "below": below,
            "above": above,
        }

    def orca_coverage_for_candidate(candidate_id: int, at_layer: int) -> dict:
        candidate = candidates.get(candidate_id)
        if candidate is None:
            return {"candidate_id": candidate_id, "known": False}
        z = z_of(at_layer)
        below = [
            plane
            for plane in orca_planes
            if plane <= z + 0.02 and z - plane <= ORCA_LOOKBACK_MM
        ]
        plane = max(below) if below else None
        segments = (
            support_segments_at(orca_geometry, plane) if plane is not None else []
        )
        shapes = [polygon_of(item) for item in candidate["geometry"]]
        area = sum(shape.area for shape in shapes)
        covers = [coverage(shape, segments) for shape in shapes]
        return {
            "candidate_id": candidate_id,
            "known": True,
            "candidate_layer": candidate["layer"],
            "candidate_z_mm": round(
                candidate["layer"] * LAYER_HEIGHT_MM + LAYER_HEIGHT_MM, 3
            ),
            "area_mm2": round(area, 3),
            "orca_plane_mm": plane,
            "orca_coverage": round(max(covers) if covers else 0.0, 4),
            "pnp_plane_mm": z,
            "pnp_coverage": 0.0,
        }

    missing = []
    rows_out = []
    for row in rejected:
        layer = row["layer"]
        estimate = neighbor_estimate(layer)
        missing.append(estimate["estimate_mm"])
        area = sum(role["area_units2"] for role in row["roles"])
        entry = {
            "layer": layer,
            "z_mm": round(row["anchor_z_mm"], 3),
            "body_area_mm2": round(area / 1e8, 3),
            "top_occupancy_area_mm2": round(
                row["top_occupancy"]["area_units2"] / 1e8, 3
            ),
            "top_overlap_mm2": round(row["top_overlap_units2"] / 1e8, 6),
            "mid_overlap_mm2": round(row["mid_overlap_units2"] / 1e8, 6),
            "overlap_pct_of_body": round(100.0 * row["top_overlap_units2"] / area, 6)
            if area
            else None,
            "roles": [
                {
                    "role": role["role"],
                    "count": role["count"],
                    "area_mm2": round(role["area_units2"] / 1e8, 3),
                    "overlap_units2": next(
                        (
                            item["overlap_units2"]
                            for item in row.get("top_overlap_by_role", [])
                            if item["role"] == role["role"]
                        ),
                        0,
                    ),
                }
                for role in row["roles"]
            ],
            "missing_path_estimate_mm": estimate["estimate_mm"],
            "neighbors": {
                "below": estimate.get("below", []),
                "above": estimate.get("above", []),
            },
        }
        if candidates:
            ids = []
            for demand in row["demand_ids"]:
                candidate_id = demand_candidate(demand)
                if candidate_id is None:
                    continue
                if candidates.get(candidate_id, {}).get("layer") == layer + 1:
                    ids.append(candidate_id)
            entry["immediate_candidates"] = [
                orca_coverage_for_candidate(candidate_id, layer)
                for candidate_id in sorted(set(ids))
            ]
        rows_out.append(entry)

    immediate_total = 0
    immediate_supported = 0
    for row in rows_out:
        for check in row.get("immediate_candidates", []):
            if not check.get("known"):
                continue
            immediate_total += 1
            if check["orca_coverage"] > 0.05:
                immediate_supported += 1

    missing_total = round(sum(missing), 3)
    body_area_total = sum(entry["body_area_mm2"] for entry in rows_out)
    return {
        "cell": label,
        "diagnostics": diagnostics,
        "rejected_entries": len(rejected),
        "retained_entries": len(retained),
        "rejected_layers": [row["layer"] for row in rows_out],
        "rejected_layers_sorted": sorted(row["layer"] for row in rows_out),
        "unique_demands_rejected": len(rejected_demands),
        "unique_demands_retained": len(retained_demands),
        "unique_demands_only_rejected": len(only_rejected),
        "support_path_mm": {
            "pnp": round(pnp_total, 3),
            "orca": round(orca_total, 3),
        },
        "missing_support_path_mm": missing_total,
        "missing_support_path_area_bound_mm": round(body_area_total / 0.42, 3),
        "missing_share_pct": round(100.0 * missing_total / pnp_total, 4)
        if pnp_total
        else None,
        "immediate_candidate_total": immediate_total,
        "immediate_candidate_orca_supported": immediate_supported,
        "entries": rows_out,
        "pnp_gcode_sha256": sha256(pnp_gcode),
        "orca_gcode_sha256": sha256(orca_gcode),
        "pnp_gcode": str(pnp_gcode.relative_to(ROOT)),
        "orca_gcode": str(orca_gcode.relative_to(ROOT)),
    }


def main() -> int:
    benchy = census_cell(
        "benchy-classic-on",
        ROOT / "target/t147/probe-benchy3/entries.jsonl",
        ROOT / "target/t147/probe-benchy3/stderr.log",
        ROOT / "target/t147/probe-benchy3/candidates.json",
        ROOT / "target/t147/benchy-classic-on.probe3.gcode",
        ROOT / "target/matched-pair/t145-benchy/gcode/benchy-classic-on-orca-m3.gcode",
        (150, 200),
    )
    base = census_cell(
        "base-classic-on",
        ROOT / "target/t147/probe-base3/entries.jsonl",
        ROOT / "target/t147/probe-base3/stderr.log",
        ROOT / "target/t147/probe-base3/candidates.json",
        ROOT / "target/t147/base-classic-on.probe3.gcode",
        ROOT / "target/matched-pair/t145-benchy/gcode/base-classic-on-orca-m3.gcode",
        (14, 42),
    )
    payload = {
        "schema": "t147-supports-rejection/v1",
        "units": "mm / mm2; overlaps are top-plane exact-Z occupancy intersections",
        "planner_plane": "layer.z - 0.5 * layer_height (layer sample midpoint)",
        "validator_plane": "layer.z (SupportPlanEntry.anchor_z / exact-Z query)",
        "support_half_width_mm": SUPPORT_HALF_WIDTH_MM,
        "layer_height_mm": LAYER_HEIGHT_MM,
        "cells": {"benchy-classic-on": benchy, "base-classic-on": base},
        "matched_config": "docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json",
    }
    out = OUT / "census.json"
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)}")
    for cell in payload["cells"].values():
        print(
            cell["cell"],
            "diag",
            cell["diagnostics"],
            "entries",
            cell["rejected_entries"],
            "layers",
            len(cell["rejected_layers"]),
            "unique-only",
            cell["unique_demands_only_rejected"],
            "missing",
            cell["missing_support_path_mm"],
            "share",
            cell["missing_share_pct"],
            "cand",
            cell["immediate_candidate_orca_supported"],
            "/",
            cell["immediate_candidate_total"],
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
