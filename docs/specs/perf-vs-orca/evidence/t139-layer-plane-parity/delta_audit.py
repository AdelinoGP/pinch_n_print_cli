#!/usr/bin/env python3
"""Ticket 139 delta audit: canonical mid-layer sampling vs the top-plane arm.

Reads the raw t139 campaign G-code (gitignored, `target/t139/output-final/`)
and the two heavy model fixtures (`tmp/3dbenchy.stl`, `tmp/base.stl`) and writes
`reduced/delta.json`, the only input `verify.py` needs.

Claims encoded here (every one re-derived from the raw artifacts):

  * the baseline ordinary arm reproduces the t135 Stage-1 reference set;
  * the candidate ordinary and accelerated arms are byte-identical in all
    eight cells;
  * the layer grid moved by a half layer, not by a rule: base gains exactly one
    layer (495 -> 496, last Z 99.0 -> 99.2); benchy stays at 240;
  * every common layer is still emitted (no gaps), and the `;TYPE:` feature set
    is unchanged per cell;
  * the layer-0 outer-wall hull now fits the STL **midpoint** section better
    than the top section on every cell, and the base fixture's steep foot makes
    that a 3x discriminator.

Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/delta_audit.py
"""

from __future__ import annotations

import collections
import hashlib
import json
import re
from pathlib import Path

import trimesh
from shapely.geometry import MultiPoint, Polygon

ROOT = Path(__file__).resolve().parents[5]
GCODE = ROOT / "target/t139/output-final2/gcode"
ROWS = ROOT / "target/t139/output-final2/rows.csv"
OUT = Path(__file__).resolve().parent / "reduced" / "delta.json"

LOOP = re.compile(
    r"^G[01] X(-?\d+(?:\.\d+)?) Y(-?\d+(?:\.\d+)?)(?:\s+Z(-?\d+(?:\.\d+)?))?(.*)$"
)
FEATURE = re.compile(r"^;TYPE:(.+?)\s*$")
E_FIELD = re.compile(r"\bE(-?(?:\d+(?:\.\d+)?|\.\d+))")

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
FIXTURES = {
    "benchy": ROOT / "tmp/3dbenchy.stl",
    "base": ROOT / "tmp/base.stl",
}
# t135 Stage-1 reference set (the pre-t139 currency), read from the t135
# candidate campaign rows (raw tree, gitignored; this reducer may cite it, the
# verifier reads only `reduced/delta.json`).
T135_ROWS = ROOT / "target/t135/output-candidate-final/rows.csv"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_cell(path: Path) -> dict:
    """Return {layers: [{z, types: {feature: loop_count}, e_mm, moves}], ...}."""
    layers = []
    z = None
    feature = None
    loops = collections.Counter()
    moves = 0
    e_total = 0.0
    layer_started = False

    def flush():
        nonlocal loops, moves, e_total, z, layer_started
        if layer_started:
            layers.append(
                {
                    "z": round(z, 4) if z is not None else None,
                    "types": dict(sorted(loops.items())),
                    "moves": moves,
                    "e_mm": round(e_total, 4),
                }
            )
        loops = collections.Counter()
        moves = 0
        e_total = 0.0
        layer_started = True

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(";LAYER_CHANGE"):
            flush()
            z = None
            feature = None
            continue
        match = re.match(r"^;Z:(-?[\d.]+)", line)
        if match:
            z = float(match.group(1))
            continue
        match = FEATURE.match(line)
        if match:
            feature = match.group(1)
            continue
        match = LOOP.match(line)
        if match and feature is not None:
            moves += 1
            # Count a run when the feature block's first move arrives.
            if loops[feature] == 0:
                loops[feature] = 1
            e = E_FIELD.search(match.group(4))
            if e:
                e_total += abs(float(e.group(1)))
    flush()
    return {"layers": layers}


def type_sections(parsed: dict) -> dict:
    """Number of layers on which each `;TYPE:` appears (role inventory)."""
    counts = collections.Counter()
    for layer in parsed["layers"]:
        for feature in layer["types"]:
            counts[feature] += 1
    return dict(sorted(counts.items()))


def layer0_outer_wall_hull(path: Path) -> float:
    """Convex-hull area (mm^2) of the layer-0 `Outer wall` path points."""
    pts = []
    feature = None
    z = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(";LAYER_CHANGE"):
            if z is not None:
                # First layer only: stop at the second change once we have points.
                if pts:
                    break
            continue
        match = re.match(r"^;Z:(-?[\d.]+)", line)
        if match:
            z = float(match.group(1))
            continue
        match = FEATURE.match(line)
        if match:
            feature = match.group(1)
            continue
        if feature == "Outer wall":
            match = LOOP.match(line)
            if match:
                pts.append((float(match.group(1)), float(match.group(2))))
    if len(pts) < 3:
        return 0.0
    return round(MultiPoint(pts).convex_hull.area, 3)


def stl_mid_section_area(path: Path, z_mid: float, inward_mm: float) -> float:
    mesh = trimesh.load(path, force="mesh")
    zmin = float(mesh.bounds[0][2])
    section = mesh.section(
        plane_origin=[0.0, 0.0, zmin + z_mid], plane_normal=[0.0, 0.0, 1.0]
    )
    if section is None:
        return 0.0
    path2d, _ = section.to_2D()
    loops = list(path2d.polygons_full)
    if not loops:
        return 0.0
    # Inward offset by the arm's outer-wall half width, then hull area: the same
    # translation-invariant metric t138 used.
    offset = []
    for poly in loops:
        eroded = poly.buffer(-inward_mm, join_style=2)
        if not eroded.is_empty:
            offset.append(eroded)
    if not offset:
        return 0.0
    pts = [pt for poly in offset for pt in poly.exterior.coords]
    return round(MultiPoint(pts).convex_hull.area, 3)


def main() -> int:
    t135 = {}
    with T135_ROWS.open(encoding="utf-8") as handle:
        header = handle.readline().strip().split(",")
        for line in handle:
            values = line.strip().split(",")
            if len(values) != len(header):
                continue
            row = dict(zip(header, values))
            t135[(row["cell"], row["arm"])] = row["sha256"]

    rows = {}
    with ROWS.open(encoding="utf-8") as handle:
        header = handle.readline().strip().split(",")
        for line in handle:
            values = line.strip().split(",")
            if len(values) != len(header):
                continue
            row = dict(zip(header, values))
            rows[(row["cell"], row["arm"])] = row

    summary = {"cells": {}, "rows": {}}
    for cell in CELLS:
        fixture = cell.split("-")[0]
        base_path = GCODE / f"{cell}-baseline-ordinary.gcode"
        cand_path = GCODE / f"{cell}-candidate-ordinary.gcode"
        acc_path = GCODE / f"{cell}-candidate-accelerated.gcode"
        base = parse_cell(base_path)
        cand = parse_cell(cand_path)
        base_z = [layer["z"] for layer in base["layers"]]
        cand_z = [layer["z"] for layer in cand["layers"]]
        base_types = {t for layer in base["layers"] for t in layer["types"]}
        cand_types = {t for layer in cand["layers"] for t in layer["types"]}
        missing = sorted(set(base_z) - set(cand_z))

        stl = FIXTURES[fixture]
        top_z = cand_z[0] if cand_z else 0.2
        # Layer 0's band midpoint (first-layer height 0.2 on the matched job).
        mid_z = round(top_z - 0.1, 6)
        summary["cells"][cell] = {
            "fixture": str(stl.relative_to(ROOT)),
            "baseline_layers": len(base_z),
            "candidate_layers": len(cand_z),
            "baseline_last_z": base_z[-1] if base_z else None,
            "candidate_last_z": cand_z[-1] if cand_z else None,
            "common_layers_missing_from_candidate": missing,
            "baseline_types": sorted(base_types),
            "candidate_types": sorted(cand_types),
            "types_equal": base_types == cand_types,
            "baseline_type_sections": type_sections(base),
            "candidate_type_sections": type_sections(cand),
            "baseline_e_mm": round(sum(layer["e_mm"] for layer in base["layers"]), 3),
            "candidate_e_mm": round(sum(layer["e_mm"] for layer in cand["layers"]), 3),
            "baseline_l0_outer_wall_hull_mm2": layer0_outer_wall_hull(base_path),
            "candidate_l0_outer_wall_hull_mm2": layer0_outer_wall_hull(cand_path),
            "stl_mid_z": mid_z,
            "stl_top_section_hull_o200_mm2": stl_mid_section_area(
                stl, round(top_z, 6), 0.20
            ),
            "stl_mid_section_hull_o200_mm2": stl_mid_section_area(stl, mid_z, 0.20),
        }
        summary["rows"][cell] = {
            "baseline_sha256": rows.get((cell, "baseline-ordinary"), {}).get("sha256"),
            "candidate_ordinary_sha256": rows.get((cell, "candidate-ordinary"), {}).get(
                "sha256"
            ),
            "candidate_accelerated_sha256": rows.get(
                (cell, "candidate-accelerated"), {}
            ).get("sha256"),
            "candidate_ordinary_sha256_on_disk": sha256(cand_path),
            "candidate_accelerated_sha256_on_disk": sha256(acc_path),
            "t135_reference_sha256": t135.get((cell, "candidate-ordinary")),
            "baseline_matches_t135_reference": rows.get(
                (cell, "baseline-ordinary"), {}
            ).get("sha256")
            == t135.get((cell, "candidate-ordinary")),
            "candidate_arms_identical": rows.get((cell, "candidate-ordinary"), {}).get(
                "sha256"
            )
            == rows.get((cell, "candidate-accelerated"), {}).get("sha256"),
        }

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    for cell, rec in summary["cells"].items():
        print(
            f"{cell}: {rec['baseline_layers']} -> {rec['candidate_layers']} layers, "
            f"last {rec['baseline_last_z']} -> {rec['candidate_last_z']}, "
            f"L0 hull {rec['baseline_l0_outer_wall_hull_mm2']} -> "
            f"{rec['candidate_l0_outer_wall_hull_mm2']} "
            f"(STL top {rec['stl_top_section_hull_o200_mm2']}, "
            f"mid {rec['stl_mid_section_hull_o200_mm2']})"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
