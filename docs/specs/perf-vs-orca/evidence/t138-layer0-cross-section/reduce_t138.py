#!/usr/bin/env python3
"""Ticket 138 reducer: layer-0 cross-section attribution.

Reads the raw t135 campaign G-code (gitignored, `target/matched-pair/t135-scoreboard3/`)
and the two heavy model fixtures (`tmp/3dbenchy.stl`, `tmp/base.stl`) and writes
`reduced/summary.json`, the only input `verify.py` needs.

No production code is touched. Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/reduce_t138.py
"""

from __future__ import annotations

import collections
import hashlib
import json
import math
import re
import sys
from pathlib import Path

import trimesh
from shapely.geometry import LineString, MultiPoint, Polygon
from shapely.ops import unary_union

ROOT = Path(__file__).resolve().parents[5]
GCODE = ROOT / "target/matched-pair/t135-scoreboard3/gcode"
OUT = Path(__file__).resolve().parent / "reduced" / "summary.json"

LOOP = re.compile(
    r"^G[01] X(-?\d+(?:\.\d+)?) Y(-?\d+(?:\.\d+)?)(?:\s+Z(-?\d+(?:\.\d+)?))?(.*)$"
)
PNP_FEATURE = re.compile(r"^;TYPE:(.+?)\s*$")
ORCA_FEATURE = re.compile(r"^; FEATURE:(.+?)\s*$")
E_FIELD = re.compile(r"\bE(-?(?:\d+(?:\.\d+)?|\.\d+))")
APPENDIX = re.compile(r"^;\s*([A-Za-z0-9_]+)\s*=\s*(.*?)\s*$")

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
    "benchy": {"stl": ROOT / "tmp/3dbenchy.stl", "sha256": None},
    "base": {"stl": ROOT / "tmp/base.stl", "sha256": None},
}
SAMPLES = {
    "benchy": [0, 10, 50, 100, 150],
    "base": [0, 10, 50, 100, 200],
}


# ---------------------------------------------------------------------------
# G-code parsing
# ---------------------------------------------------------------------------
def parse_gcode(path: Path, arm: str) -> dict:
    """Return {layer_index: {feature: [run]}}, run = {"pts", "travel"}."""
    marker = ";LAYER_CHANGE" if arm == "pnp" else "; CHANGE_LAYER"
    feature_re = PNP_FEATURE if arm == "pnp" else ORCA_FEATURE
    z_re = (
        re.compile(r"^;Z:(-?[\d.]+)")
        if arm == "pnp"
        else re.compile(r"^; Z_HEIGHT: (-?[\d.]+)")
    )
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    starts = [i for i, line in enumerate(lines) if line.startswith(marker)]
    layers: dict = {}
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        z = None
        feats: dict = {}
        feat = None
        run = None
        travel = None

        def flush():
            nonlocal run
            if run is not None and len(run["pts"]) >= 3:
                feats.setdefault(feat, []).append(run)
            run = None

        for line in lines[start:end]:
            zm = z_re.match(line)
            if zm:
                z = float(zm.group(1))
                continue
            fm = feature_re.match(line)
            if fm:
                flush()
                travel = None
                feat = fm.group(1).strip()
                continue
            lm = LOOP.match(line)
            if not lm or feat is None:
                continue
            x, y = float(lm.group(1)), float(lm.group(2))
            e = E_FIELD.search(lm.group(4))
            if e is None:
                # Travel: close the current extrusion run.
                flush()
                if arm == "pnp":
                    travel = (x, y)
                continue
            if e.group(1).startswith("-"):
                # Retraction: not laid-down plastic, but it does not split the
                # feature section (the t135 oracle counts runs the same way).
                continue
            if run is None:
                run = {"pts": [], "travel": travel if arm == "pnp" else None}
            run["pts"].append((x, y))
        flush()
        layers[n] = {"z": z, "features": feats}
    return layers


def closed_points(run: dict) -> list:
    return ([run["travel"]] if run.get("travel") else []) + run["pts"]


def run_length(run: dict) -> float:
    pts = run["pts"]
    return sum(math.dist(pts[i], pts[i + 1]) for i in range(len(pts) - 1))


def shoelace(points: list) -> float:
    n = len(points)
    if n < 3:
        return 0.0
    twice = 0.0
    for i in range(n):
        x0, y0 = points[i]
        x1, y1 = points[(i + 1) % n]
        twice += x0 * y1 - x1 * y0
    return abs(twice) * 0.5


def feature_metrics(runs: list, spacing: float) -> dict:
    if not runs:
        return {
            "present": False,
            "runs": 0,
            "path_mm": 0.0,
            "swept_mm2": 0.0,
            "outer_loop_mm2": 0.0,
            "inner_loop_mm2": 0.0,
        }
    areas = [shoelace(closed_points(r)) for r in runs]
    areas = [a for a in areas if a > 1e-6]
    path = sum(run_length(r) for r in runs)
    pts = [p for r in runs for p in closed_points(r)]
    return {
        "present": True,
        "runs": len(runs),
        "path_mm": round(path, 3),
        "swept_mm2": round(path * spacing, 3),
        "outer_loop_mm2": round(max(areas), 3) if areas else 0.0,
        "inner_loop_mm2": round(min(areas), 3) if areas else 0.0,
        "hull_mm2": round(MultiPoint(pts).convex_hull.area, 3)
        if len(pts) >= 3
        else 0.0,
        "bbox": [
            round(v, 3)
            for v in (
                min(p[0] for p in pts),
                min(p[1] for p in pts),
                max(p[0] for p in pts),
                max(p[1] for p in pts),
            )
        ],
    }


def support_metrics(runs: list, bead: float) -> dict:
    if not runs:
        return {
            "present": False,
            "runs": 0,
            "path_mm": 0.0,
            "hull_mm2": 0.0,
            "coverage_mm2": 0.0,
        }
    pts = [p for r in runs for p in r["pts"]]
    path = sum(run_length(r) for r in runs)
    lines = [LineString(r["pts"]) for r in runs if len(r["pts"]) >= 2]
    coverage = (
        unary_union([ln.buffer(bead, cap_style=2, join_style=2) for ln in lines]).area
        if lines
        else 0.0
    )
    return {
        "present": True,
        "runs": len(runs),
        "path_mm": round(path, 3),
        "hull_mm2": round(MultiPoint(pts).convex_hull.area, 3)
        if len(pts) >= 3
        else 0.0,
        "coverage_mm2": round(coverage, 3),
    }


def union_hull(feats: dict) -> float:
    """Convex hull of `_make_skirt`'s input: every non-Skirt point plus the brim."""
    pts = [
        p
        for role, runs in feats.items()
        if role != "Skirt"
        for r in runs
        for p in closed_points(r)
    ]
    return round(MultiPoint(pts).convex_hull.area, 3) if len(pts) >= 3 else 0.0


def growth(ref: list, box: list) -> dict:
    return {
        "xmin": ref[0] - box[0],
        "ymin": ref[1] - box[1],
        "xmax": box[2] - ref[2],
        "ymax": box[3] - ref[3],
    }


# ---------------------------------------------------------------------------
# STL cross-sections
# ---------------------------------------------------------------------------
def stl_loops(mesh, z: float) -> list:
    segs = trimesh.intersections.mesh_plane(
        mesh, plane_normal=(0, 0, 1), plane_origin=(0, 0, z)
    )

    def key(p):
        return (round(float(p[0]), 6), round(float(p[1]), 6))

    adj = collections.defaultdict(set)
    pairs = []
    for seg in segs:
        a, b = key(seg[0]), key(seg[1])
        if a == b:
            continue
        adj[a].add(b)
        adj[b].add(a)
        pairs.append((a, b))
    used = set()
    loops = []
    for a, b in pairs:
        e0 = tuple(sorted((a, b)))
        if e0 in used:
            continue
        used.add(e0)
        chain = [a, b]
        prev, cur = a, b
        while True:
            nxts = [p for p in adj[cur] if p != prev]
            if not nxts:
                break
            nxt = nxts[0]
            if nxt == chain[0]:
                used.add(tuple(sorted((cur, nxt))))
                break
            e2 = tuple(sorted((cur, nxt)))
            if e2 in used:
                break
            used.add(e2)
            chain.append(nxt)
            prev, cur = cur, nxt
        if len(chain) >= 4:
            loops.append(chain)
    return loops


def offset_polygons(loops: list, d: float) -> list:
    out = []
    for chain in loops:
        p = Polygon(chain)
        if not p.is_valid:
            p = p.buffer(0)
        q = p.buffer(-d, join_style=2, mitre_limit=3.0)
        for g in [q] if q.geom_type == "Polygon" else getattr(q, "geoms", []):
            if g.area > 1e-9:
                out.append(g)
    return out


def offset_hull_area(loops: list, d: float) -> float:
    pts = []
    for g in offset_polygons(loops, d):
        pts.extend(list(g.exterior.coords))
    return round(MultiPoint(pts).convex_hull.area, 3) if len(pts) >= 3 else 0.0


def raw_area(loops: list) -> float:
    total = 0.0
    for chain in loops:
        p = Polygon(chain)
        total += p.area if p.is_valid else p.buffer(0).area
    return round(total, 4)


def raw_perimeter(loops: list) -> float:
    return round(sum(Polygon(c).length for c in loops), 4)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main() -> int:
    if not GCODE.is_dir():
        print(f"missing raw campaign tree: {GCODE}", file=sys.stderr)
        return 2
    for fixture in FIXTURES.values():
        if not fixture["stl"].is_file():
            print(f"missing model fixture: {fixture['stl']}", file=sys.stderr)
            return 2

    meshes = {}
    for name, fixture in FIXTURES.items():
        mesh = trimesh.load(fixture["stl"], process=False)
        fixture["sha256"] = hashlib.sha256(fixture["stl"].read_bytes()).hexdigest()
        fixture["faces"] = len(mesh.faces)
        fixture["bounds"] = [[round(v, 6) for v in row] for row in mesh.bounds.tolist()]
        fixture["min_z"] = fixture["bounds"][0][2]
        fixture["height_mm"] = round(fixture["bounds"][1][2] - fixture["min_z"], 6)
        meshes[name] = mesh

    gcode = {}
    for cell in CELLS:
        fixture = cell.split("-")[0]
        pnp_path = GCODE / f"{cell}-pnp-ordinary-m3.gcode"
        orca_path = GCODE / f"{cell}-orca-m3.gcode"
        pnp_layers = parse_gcode(pnp_path, "pnp")
        orca_layers = parse_gcode(orca_path, "orca")
        pnp_cfg = {}
        for line in pnp_path.read_text(encoding="utf-8", errors="replace").splitlines():
            m = APPENDIX.match(line)
            if m:
                pnp_cfg.setdefault(m.group(1), m.group(2))
        orca_cfg = {}
        for line in orca_path.read_text(
            encoding="utf-8", errors="replace"
        ).splitlines():
            m = APPENDIX.match(line)
            if m:
                orca_cfg.setdefault(m.group(1), m.group(2))
        gcode[cell] = {
            "fixture": fixture,
            "pnp": pnp_layers,
            "orca": orca_layers,
            "pnp_cfg": pnp_cfg,
            "orca_cfg": orca_cfg,
        }

    # --- layer counts -------------------------------------------------------
    layer_counts = {}
    for fixture in FIXTURES:
        pnp_cell = gcode[f"{fixture}-classic-off"]["pnp"]
        orca_cell = gcode[f"{fixture}-classic-off"]["orca"]
        layer_counts[fixture] = {
            "pnp_layers": len(pnp_cell),
            "orca_layers": len(orca_cell),
            "pnp_last_z": max(v["z"] for v in pnp_cell.values() if v["z"] is not None),
            "orca_last_z": max(
                v["z"] for v in orca_cell.values() if v["z"] is not None
            ),
            "object_height_mm": FIXTURES[fixture]["height_mm"],
        }

    # --- slice-plane discriminator -----------------------------------------
    plane_samples = []
    for fixture, layers in SAMPLES.items():
        mesh = meshes[fixture]
        min_z = FIXTURES[fixture]["min_z"]
        pnp_cell = gcode[f"{fixture}-classic-off"]["pnp"]
        orca_cell = gcode[f"{fixture}-classic-off"]["orca"]
        for n in layers:
            z_top = pnp_cell[n]["z"]
            z_mid = round(z_top - 0.1, 6)
            stl_top = stl_loops(mesh, min_z + z_top)
            stl_mid = stl_loops(mesh, min_z + z_mid)
            pnp_hull = feature_metrics(
                pnp_cell[n]["features"].get("Outer wall", []), 0.35708
            )["hull_mm2"]
            orca_hull = feature_metrics(
                orca_cell[n]["features"].get("Outer wall", []), 0.35708
            )["hull_mm2"]
            rec = {
                "fixture": fixture,
                "layer": n,
                "z_top": z_top,
                "z_mid": z_mid,
                "pnp_outer_wall_hull_mm2": pnp_hull,
                "orca_outer_wall_hull_mm2": orca_hull,
                "stl_top_o200_mm2": offset_hull_area(stl_top, 0.20),
                "stl_top_o210_mm2": offset_hull_area(stl_top, 0.21),
                "stl_mid_o200_mm2": offset_hull_area(stl_mid, 0.20),
                "stl_mid_o210_mm2": offset_hull_area(stl_mid, 0.21),
            }

            def rel(obs, pred):
                return round(abs(obs - pred) / pred, 4) if pred else None

            rec["pnp_vs_top"] = rel(pnp_hull, rec["stl_top_o200_mm2"])
            rec["pnp_vs_mid"] = rel(pnp_hull, rec["stl_mid_o210_mm2"])
            rec["orca_vs_top"] = rel(orca_hull, rec["stl_top_o210_mm2"])
            rec["orca_vs_mid"] = rel(orca_hull, rec["stl_mid_o210_mm2"])
            # "Discriminating" when the two candidate planes differ by >10 % in
            # the compared metric, so a 1-2 % match is not a coincidence.
            top_pred = rec["stl_top_o200_mm2"]
            mid_pred = rec["stl_mid_o210_mm2"]
            rec["plane_separation"] = (
                round(abs(top_pred - mid_pred) / min(top_pred, mid_pred), 4)
                if min(top_pred, mid_pred)
                else None
            )
            rec["discriminating"] = bool(
                rec["plane_separation"] and rec["plane_separation"] > 0.10
            )
            plane_samples.append(rec)

    # --- wall-width at a common plane --------------------------------------
    wall_width = {}
    for fixture, layer in (("base", 0), ("benchy", 0)):
        mesh = meshes[fixture]
        min_z = FIXTURES[fixture]["min_z"]
        cell = gcode[f"{fixture}-classic-off"]
        z_top = cell["pnp"][layer]["z"]
        stl_top = stl_loops(mesh, min_z + z_top)
        pnp_hull = feature_metrics(
            cell["pnp"][layer]["features"].get("Outer wall", []), 0.35708
        )["hull_mm2"]
        orca_hull = feature_metrics(
            cell["orca"][layer]["features"].get("Outer wall", []), 0.35708
        )["hull_mm2"]
        top200 = offset_hull_area(stl_top, 0.20)
        top210 = offset_hull_area(stl_top, 0.21)
        wall_width[fixture] = {
            "layer": layer,
            "z_top": z_top,
            "pnp_hull_mm2": pnp_hull,
            "orca_hull_mm2": orca_hull,
            "observed_ratio": round(pnp_hull / orca_hull, 4) if orca_hull else None,
            "stl_top_o200_mm2": top200,
            "stl_top_o210_mm2": top210,
            "same_plane_wall_width_effect": round(abs(top200 - top210) / top210, 4)
            if top210
            else None,
        }

    # --- brim / anchor per cell --------------------------------------------
    brim = {}
    for cell in CELLS:
        pnp = gcode[cell]["pnp"]
        orca = gcode[cell]["orca"]
        p_spacing = 0.35708
        o_spacing = 0.35708
        p = feature_metrics(pnp[0]["features"].get("Brim", []), p_spacing)
        o = feature_metrics(orca[0]["features"].get("Brim", []), o_spacing)
        ps = feature_metrics(pnp[0]["features"].get("Skirt", []), p_spacing)
        os = feature_metrics(orca[0]["features"].get("Skirt", []), o_spacing)
        po = feature_metrics(pnp[0]["features"].get("Outer wall", []), p_spacing)
        oo = feature_metrics(orca[0]["features"].get("Outer wall", []), o_spacing)
        brim[cell] = {
            "pnp": p,
            "orca": o,
            "pnp_skirt": ps,
            "orca_skirt": os,
            "pnp_outer_wall": po,
            "orca_outer_wall": oo,
            "brim_swept_rel_diff": round(
                abs(p["swept_mm2"] - o["swept_mm2"]) / o["swept_mm2"], 4
            )
            if o["swept_mm2"]
            else None,
            "skirt_outer_loop_rel_diff": round(
                abs(ps["outer_loop_mm2"] - os["outer_loop_mm2"]) / os["outer_loop_mm2"],
                4,
            )
            if os["outer_loop_mm2"]
            else None,
            "pnp_brim_growth": growth(po["bbox"], p["bbox"]) if p["present"] else None,
            "orca_brim_growth": growth(oo["bbox"], o["bbox"]) if o["present"] else None,
        }

    # --- anchor decomposition (base supports-off) --------------------------
    base_off = brim["base-classic-off"]
    mesh = meshes["base"]
    min_z = FIXTURES["base"]["min_z"]
    z_top = gcode["base-classic-off"]["pnp"][0]["z"]
    stl_top = stl_loops(mesh, min_z + z_top)
    stl_mid = stl_loops(mesh, min_z + (z_top - 0.1))
    loops = base_off["pnp"]["runs"]
    contour_top = raw_perimeter(stl_top)
    contour_mid = raw_perimeter(stl_mid)
    contour_term = loops * (contour_top - contour_mid)
    anchor_term = -loops * (2.0 * math.pi * 0.20)
    predicted = contour_term + anchor_term
    observed = base_off["pnp"]["path_mm"] - base_off["orca"]["path_mm"]
    anchor = {
        "cell": "base-classic-off",
        "loops": loops,
        "pnp_brim_path_mm": base_off["pnp"]["path_mm"],
        "orca_brim_path_mm": base_off["orca"]["path_mm"],
        "observed_delta_mm": round(observed, 3),
        "stl_contour_perimeter_top_mm": contour_top,
        "stl_contour_perimeter_mid_mm": contour_mid,
        "contour_perimeter_delta_mm": round(contour_top - contour_mid, 4),
        "slice_plane_term_mm": round(contour_term, 3),
        "anchor_term_mm": round(anchor_term, 3),
        "predicted_delta_mm": round(predicted, 3),
        "residual_mm": round(observed - predicted, 3),
        "residual_rel": round(abs(observed - predicted) / abs(predicted), 4)
        if predicted
        else None,
        "anchor_note": "2*pi*r approximation for the 0.20 mm wall-path anchor shift",
    }

    # --- supports-on: support footprint and skirt input --------------------
    supports_on = {}
    for cell in (
        "base-classic-on",
        "base-arachne-on",
        "benchy-classic-on",
        "benchy-arachne-on",
    ):
        pnp = gcode[cell]["pnp"]
        orca = gcode[cell]["orca"]
        p_sup = support_metrics(pnp[0]["features"].get("Support", []), 0.20)
        o_sup = support_metrics(orca[0]["features"].get("Support", []), 0.21)
        p_union = union_hull(pnp[0]["features"])
        o_union = union_hull(orca[0]["features"])
        p_skirt = brim[cell]["pnp_skirt"]["outer_loop_mm2"]
        o_skirt = brim[cell]["orca_skirt"]["outer_loop_mm2"]
        supports_on[cell] = {
            "pnp_support": p_sup,
            "orca_support": o_sup,
            "support_path_ratio": round(o_sup["path_mm"] / p_sup["path_mm"], 4)
            if p_sup["path_mm"]
            else None,
            "pnp_union_hull_mm2": p_union,
            "orca_union_hull_mm2": o_union,
            "union_hull_ratio": round(p_union / o_union, 4) if o_union else None,
            "pnp_skirt_outer_loop_mm2": p_skirt,
            "orca_skirt_outer_loop_mm2": o_skirt,
            "skirt_ratio": round(p_skirt / o_skirt, 4) if o_skirt else None,
            "pnp_brim": brim[cell]["pnp"],
            "orca_brim": brim[cell]["orca"],
        }

    # --- supports-off: the same skirt-input check without the support term --
    supports_off = {}
    for cell in (
        "base-classic-off",
        "base-arachne-off",
        "benchy-classic-off",
        "benchy-arachne-off",
    ):
        pnp = gcode[cell]["pnp"]
        orca = gcode[cell]["orca"]
        p_union = union_hull(pnp[0]["features"])
        o_union = union_hull(orca[0]["features"])
        p_skirt = brim[cell]["pnp_skirt"]["outer_loop_mm2"]
        o_skirt = brim[cell]["orca_skirt"]["outer_loop_mm2"]
        supports_off[cell] = {
            "pnp_union_hull_mm2": p_union,
            "orca_union_hull_mm2": o_union,
            "union_hull_ratio": round(p_union / o_union, 4) if o_union else None,
            "pnp_skirt_outer_loop_mm2": p_skirt,
            "orca_skirt_outer_loop_mm2": o_skirt,
            "skirt_ratio": round(p_skirt / o_skirt, 4) if o_skirt else None,
        }

    # --- t135 oracle verdicts (from the recorded oracle JSONs) -------------
    oracle = {}
    oracle_dir = ROOT / "target/t135/oracle-final"
    for cell in CELLS:
        path = oracle_dir / f"{cell}.json"
        if not path.is_file():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        oracle[cell] = {
            "pass": data["checks"]["pass"],
            "checks": {
                k: v
                for k, v in data["checks"].items()
                if k.endswith("_pass") or k in ("pass",)
            },
            "brim_swept_area_rel_diff": data["checks"]["brim_swept_area_rel_diff"],
            "skirt_outer_loop_area_rel_diff": data["checks"][
                "skirt_outer_loop_area_rel_diff"
            ],
            "brim_growth_abs_diff_mm": data["checks"]["brim_growth_abs_diff_mm"],
            "skirt_growth_abs_diff_mm": data["checks"]["skirt_growth_abs_diff_mm"],
            "pnp_brim_loops": data["pnp"]["Brim"]["loop_count"],
            "orca_brim_loops": data["orca"]["Brim"]["loop_count"],
        }
    oracle_pass = sum(1 for v in oracle.values() if v["pass"])

    # --- price --------------------------------------------------------------
    base_top = stl_loops(meshes["base"], FIXTURES["base"]["min_z"] + 0.2)
    base_mid = stl_loops(meshes["base"], FIXTURES["base"]["min_z"] + 0.1)
    benchy_top = stl_loops(meshes["benchy"], FIXTURES["benchy"]["min_z"] + 0.2)
    benchy_mid = stl_loops(meshes["benchy"], FIXTURES["benchy"]["min_z"] + 0.1)
    price = {
        "required_cut_pct_range": [59.1, 73.5],
        "required_cut_source": "t135 Stage 1 scoreboard, 2.445x-3.769x ordinary / 2.420x-3.745x accelerated (re-derive at use)",
        "layer0_raw_contour_area_mm2": {
            "base_top": raw_area(base_top),
            "base_mid": raw_area(base_mid),
            "benchy_top": raw_area(benchy_top),
            "benchy_mid": raw_area(benchy_mid),
        },
        "layer0_plane_ratio": {
            "base": round(raw_area(base_top) / raw_area(base_mid), 4)
            if raw_area(base_mid)
            else None,
            "benchy": round(raw_area(benchy_top) / raw_area(benchy_mid), 4)
            if raw_area(benchy_mid)
            else None,
        },
        "layer_fraction": {
            fixture: round(1.0 / layer_counts[fixture]["pnp_layers"], 6)
            for fixture in FIXTURES
        },
        "note": "Sampling-plane change, not a work-set reduction. The layer-0 "
        "amplification is one layer of 495 PnP / 496 Orca (base) or 240 "
        "(benchy); no wall claim is taken.",
    }

    summary = {
        "ticket": 138,
        "generated_by": "reduce_t138.py",
        "raw_gcode": str(GCODE.relative_to(ROOT)),
        "reference_set": "t135 Stage 1 (2026-11-10)",
        "fixtures": {
            k: {
                kk: (
                    str(vv.relative_to(ROOT))
                    if isinstance(vv, Path) and vv.is_absolute()
                    else vv
                )
                for kk, vv in v.items()
            }
            for k, v in FIXTURES.items()
        },
        "layer_counts": layer_counts,
        "slice_plane": {"samples": plane_samples},
        "wall_width": wall_width,
        "brim": brim,
        "anchor": anchor,
        "supports_on": supports_on,
        "supports_off": supports_off,
        "oracle": {
            "cells": oracle,
            "pass_count": oracle_pass,
            "cell_count": len(oracle),
        },
        "price": price,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
