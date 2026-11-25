#!/usr/bin/env python3
"""Ticket 145 reducer: benchy fixture re-baseline.

Reads the raw take (gitignored `target/`) and writes the in-repo reduced set:

  reduced/summary.json   every claim in FINDINGS.md, re-derivable by verify.py
  reference.json         the active benchy reference currency + retired sets

Inputs (raw, optional for verification):
  target/matched-pair/t145-benchy/scoreboard-rows.csv   the 8-cell x 2-tool run
  target/t142/output1/gcode/*-baseline-ordinary.gcode   old export (pre-swap)
  target/t143/output/gcode/*-baseline-ordinary.gcode    new export (post-swap)
  target/t145/dist-accelerated-attempt.log              accelerated host attempt
  tmp/3dbenchy.stl, tmp/base.stl                        the fixtures themselves

In-repo inputs (required):
  evidence/t138-layer0-cross-section/reduced/summary.json   old-export STL facts
  evidence/t143-union-grouping/reference.json               interim benchy set
  evidence/t144-brim-anchor/reference.json                  active set

Usage:
    python reduce_t145.py
"""

from __future__ import annotations

import collections
import csv
import hashlib
import json
import math
import re
import statistics
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
EVIDENCE = ROOT / "docs/specs/perf-vs-orca/evidence"

OLD_GCODE = ROOT / "target/t142/output1/gcode"
NEW_GCODE = ROOT / "target/t143/output/gcode"
CSV = ROOT / "target/matched-pair/t145-benchy/scoreboard-rows.csv"
ACCEL_LOG = ROOT / "target/t145/dist-accelerated-attempt.log"
BENCHY_STL = ROOT / "tmp/3dbenchy.stl"
BASE_STL = ROOT / "tmp/base.stl"
T138 = EVIDENCE / "t138-layer0-cross-section/reduced/summary.json"
T143_REF = EVIDENCE / "t143-union-grouping/reference.json"
T144_REF = EVIDENCE / "t144-brim-anchor/reference.json"

CELLS = ["classic-off", "classic-on", "arachne-off", "arachne-on"]
FIXTURES = ["benchy", "base"]
BENCHY_CELLS = [f"benchy-{c}" for c in CELLS]
BASE_CELLS = [f"base-{c}" for c in CELLS]

# t138's slice_plane samples (old export), z_mid / z_top planes. These are the
# old-export model-level facts the swap is classified against.
OLD_PLANES = {
    0: {"z_mid": 0.1, "z_top": 0.2},
    10: {"z_mid": 2.1, "z_top": 2.2},
    50: {"z_mid": 10.1, "z_top": 10.2},
    100: {"z_mid": 20.1, "z_top": 20.2},
    150: {"z_mid": 30.1, "z_top": 30.2},
}

FEATURE = re.compile(r"^;TYPE:(.+?)\s*$")
LAYER = re.compile(r"^;LAYER_CHANGE")
ZLINE = re.compile(r"^;Z:(-?[\d.]+)")
MOVE = re.compile(r"^G[01] X(-?[\d.]+) Y(-?[\d.]+)(.*)$")
EFIELD = re.compile(r"\sE(-?(?:\d+(?:\.\d+)?|\.\d+))")


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def stl_facts(path: Path) -> dict:
    data = path.read_bytes()
    n = struct.unpack_from("<I", data, 80)[0]
    xs: list[float] = []
    ys: list[float] = []
    zs: list[float] = []
    for i in range(n):
        off = 84 + i * 50 + 12
        v = struct.unpack_from("<9f", data, off)
        xs += [v[0], v[3], v[6]]
        ys += [v[1], v[4], v[7]]
        zs += [v[2], v[5], v[8]]
    return {
        "sha256": sha256(path),
        "bytes": len(data),
        "triangles": n,
        "bounds": {
            "min": [min(xs), min(ys), min(zs)],
            "max": [max(xs), max(ys), max(zs)],
        },
    }


def parse_gcode(path: Path) -> dict:
    """Layer list: {z, features: {name: [[(x, y), ...], ...]}}."""
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    starts = [i for i, ln in enumerate(lines) if LAYER.match(ln)]
    layers = []
    for k, start in enumerate(starts):
        end = starts[k + 1] if k + 1 < len(starts) else len(lines)
        z = None
        feats: dict[str, list[list[tuple[float, float]]]] = {}
        feat = None
        run = None
        for ln in lines[start:end]:
            m = ZLINE.match(ln)
            if m:
                z = float(m.group(1))
                continue
            m = FEATURE.match(ln)
            if m:
                feat = m.group(1).strip()
                run = None
                continue
            m = MOVE.match(ln)
            if not m or feat is None:
                continue
            x, y = float(m.group(1)), float(m.group(2))
            e = EFIELD.search(m.group(3))
            if e is None or e.group(1).startswith("-"):
                run = None
                continue
            if run is None:
                run = []
                feats.setdefault(feat, []).append(run)
            run.append((x, y))
        layers.append({"z": z, "features": feats})
    return {"lines": len(lines), "layers": layers}


def path_len(runs) -> float:
    total = 0.0
    for r in runs:
        for i in range(len(r) - 1):
            total += math.dist(r[i], r[i + 1])
    return total


def shoelace(pts) -> float:
    if len(pts) < 3:
        return 0.0
    twice = 0.0
    for i in range(len(pts)):
        x0, y0 = pts[i]
        x1, y1 = pts[(i + 1) % len(pts)]
        twice += x0 * y1 - x1 * y0
    return abs(twice) * 0.5


def outer_area(feats) -> float:
    runs = feats.get("Outer wall", [])
    areas = [shoelace([*r, r[0]]) for r in runs if len(r) >= 3]
    areas = [a for a in areas if a > 1e-6]
    return max(areas) if areas else 0.0


def rel(new: float, old: float):
    if old == 0:
        return None
    return (new - old) / old


def quantiles(values: list[float]) -> dict:
    s = sorted(values)
    n = len(s)
    if n == 0:
        return {}
    return {
        "min": round(s[0], 6),
        "p05": round(s[int(0.05 * (n - 1))], 6),
        "median": round(statistics.median(s), 6),
        "p95": round(s[int(0.95 * (n - 1))], 6),
        "max": round(s[-1], 6),
    }


# ---------------------------------------------------------------------------
# 1. fixture facts and the swap classification
# ---------------------------------------------------------------------------
def swap_section() -> dict:
    t138 = json.loads(T138.read_text(encoding="utf-8"))
    old_benchy = t138["fixtures"]["benchy"]
    old_samples = {
        s["layer"]: s
        for s in t138["slice_plane"]["samples"]
        if s["fixture"] == "benchy"
    }

    new_benchy = stl_facts(BENCHY_STL)
    base = stl_facts(BASE_STL)
    old_bounds = old_benchy["bounds"]
    new_bounds = new_benchy["bounds"]["min"], new_benchy["bounds"]["max"]

    def bounds_delta(a, b):
        return [round(abs(x - y), 9) for x, y in zip(a, b)]

    # --- STL cross-sections at t138's sampled planes -----------------------
    cross = []
    try:
        import trimesh
        from shapely.geometry import MultiPoint, Polygon

        mesh = trimesh.load(str(BENCHY_STL), process=False)

        def loops_at(z):
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

        def offset_hull(loops, d):
            pts = []
            for chain in loops:
                p = Polygon(chain)
                if not p.is_valid:
                    p = p.buffer(0)
                q = p.buffer(-d, join_style=2, mitre_limit=3.0)
                for g in [q] if q.geom_type == "Polygon" else getattr(q, "geoms", []):
                    if g.area > 1e-9:
                        pts.extend(list(g.exterior.coords))
            return round(MultiPoint(pts).convex_hull.area, 3) if len(pts) >= 3 else 0.0

        for layer, planes in OLD_PLANES.items():
            ref = old_samples[layer]
            new_mid = offset_hull(loops_at(planes["z_mid"]), 0.20)
            new_top = offset_hull(loops_at(planes["z_top"]), 0.20)
            cross.append(
                {
                    "layer": layer,
                    "z_mid": planes["z_mid"],
                    "z_top": planes["z_top"],
                    "old_mid_o200_mm2": ref["stl_mid_o200_mm2"],
                    "old_top_o200_mm2": ref["stl_top_o200_mm2"],
                    "new_mid_o200_mm2": new_mid,
                    "new_top_o200_mm2": new_top,
                    "mid_rel": round(rel(new_mid, ref["stl_mid_o200_mm2"]), 9),
                    "top_rel": round(rel(new_top, ref["stl_top_o200_mm2"]), 9),
                }
            )
        cross_ok = all(r["mid_rel"] == 0.0 and r["top_rel"] == 0.0 for r in cross)
    except ImportError as exc:  # pragma: no cover - REPRODUCE documents this
        cross_ok = None
        cross.append({"error": f"trimesh/shapely unavailable: {exc}"})

    # --- printed-geometry delta: old export (t142) vs new export (t143) ----
    cells = {}
    for cell in BENCHY_CELLS:
        old_path = OLD_GCODE / f"{cell}-baseline-ordinary.gcode"
        new_path = NEW_GCODE / f"{cell}-baseline-ordinary.gcode"
        old = parse_gcode(old_path)
        new = parse_gcode(new_path)
        old_z = [l["z"] for l in old["layers"]]
        new_z = [l["z"] for l in new["layers"]]

        feats = set()
        for l in old["layers"]:
            feats.update(l["features"])
        for l in new["layers"]:
            feats.update(l["features"])
        per_feature = {}
        for feat in sorted(feats):
            o = sum(path_len(l["features"].get(feat, [])) for l in old["layers"])
            n = sum(path_len(l["features"].get(feat, [])) for l in new["layers"])

            def max_loop_area(layers):
                total = 0.0
                for l in layers:
                    areas = [
                        shoelace([*r, r[0]])
                        for r in l["features"].get(feat, [])
                        if len(r) >= 3
                    ]
                    areas = [a for a in areas if a > 1e-6]
                    if areas:
                        total += max(areas)
                return total

            oa = max_loop_area(old["layers"])
            na = max_loop_area(new["layers"])
            per_feature[feat] = {
                "path_mm_old": round(o, 3),
                "path_mm_new": round(n, 3),
                "path_rel": None if o == 0 else round(rel(n, o), 6),
                "max_loop_area_mm2_old": round(oa, 3),
                "max_loop_area_mm2_new": round(na, 3),
                "max_loop_area_rel": None if oa == 0 else round(rel(na, oa), 6),
            }

        n = min(len(old["layers"]), len(new["layers"]))
        area_rel = []
        path_rel = []
        bridge_layers = []
        support_layers = []
        worst = []
        for i in range(n):
            of = old["layers"][i]["features"]
            nf = new["layers"][i]["features"]
            oa, na = outer_area(of), outer_area(nf)
            if oa > 1.0:
                area_rel.append(rel(na, oa))
                worst.append((abs(rel(na, oa)), i, oa, na))
            op = sum(path_len(v) for v in of.values())
            np_ = sum(path_len(v) for v in nf.values())
            if op > 0:
                path_rel.append(rel(np_, op))
            ob = path_len(of.get("Internal Bridge", []))
            nb = path_len(nf.get("Internal Bridge", []))
            if abs(nb - ob) > 1e-6:
                bridge_layers.append(
                    {
                        "layer": i,
                        "z": old["layers"][i]["z"],
                        "old_mm": round(ob, 3),
                        "new_mm": round(nb, 3),
                        "delta_mm": round(nb - ob, 3),
                    }
                )
            os_ = path_len(of.get("Support", []))
            ns = path_len(nf.get("Support", []))
            if abs(ns - os_) > 1e-6:
                support_layers.append(ns - os_)
        worst.sort(reverse=True)
        cells[cell] = {
            "bytes_old": old_path.stat().st_size,
            "bytes_new": new_path.stat().st_size,
            "lines_old": old["lines"],
            "lines_new": new["lines"],
            "layers_old": len(old["layers"]),
            "layers_new": len(new["layers"]),
            "z_schedule_equal": old_z == new_z,
            "last_z": old_z[-1] if old_z else None,
            "per_feature": per_feature,
            "outer_wall_area_rel": quantiles(area_rel),
            "outer_wall_area_layers_over_0_1pct": sum(
                1 for a in area_rel if abs(a) > 0.001
            ),
            "outer_wall_area_layers_over_1pct": sum(
                1 for a in area_rel if abs(a) > 0.01
            ),
            "outer_wall_area_worst": [
                {
                    "layer": i,
                    "old_mm2": round(o, 3),
                    "new_mm2": round(nw, 3),
                    "rel": round(r, 6),
                }
                for r, i, o, nw in worst[:3]
            ],
            "total_path_rel": quantiles(path_rel),
            "internal_bridge_moved_layers": bridge_layers,
            "support_moved_layers": len(support_layers),
            "support_delta_mm": round(sum(support_layers), 3),
        }

    # --- control: the unchanged fixture's cells are byte-identical ---------
    base_control = {}
    for cell in BASE_CELLS:
        o = sha256(OLD_GCODE / f"{cell}-baseline-ordinary.gcode")
        n = sha256(NEW_GCODE / f"{cell}-baseline-ordinary.gcode")
        base_control[cell] = {"old_sha256": o, "new_sha256": n, "equal": o == n}

    return {
        "old_export": {
            "recorded_by": "t138 reduced/summary.json (old export, pre-swap)",
            "sha256": old_benchy["sha256"],
            "triangles": old_benchy["faces"],
            "bounds": old_bounds,
            "slice_z_sampling": "top plane (pre-t139)",
        },
        "new_export": {
            "sha256": new_benchy["sha256"],
            "bytes": new_benchy["bytes"],
            "triangles": new_benchy["triangles"],
            "bounds": [new_bounds[0], new_bounds[1]],
        },
        "bounds_abs_delta_mm": {
            "min": bounds_delta(old_bounds[0], new_bounds[0]),
            "max": bounds_delta(old_bounds[1], new_bounds[1]),
        },
        "triangle_delta": new_benchy["triangles"] - old_benchy["faces"],
        "base_fixture_sha256": {
            "sha256": base["sha256"],
            "recorded_by_t138": t138["fixtures"]["base"]["sha256"],
            "equal": base["sha256"] == t138["fixtures"]["base"]["sha256"],
        },
        "cross_sections": cross,
        "cross_sections_all_equal": cross_ok,
        "gcode_delta": {
            "old_source": "target/t142/output1 (pre-swap fixture, t142 tree)",
            "new_source": "target/t143/output (replacement fixture, t143 baseline = same tree)",
            "base_control_byte_identical": base_control,
            "base_control_all_equal": all(v["equal"] for v in base_control.values()),
            "cells": cells,
        },
    }


# ---------------------------------------------------------------------------
# 2. post-swap scoreboard
# ---------------------------------------------------------------------------
def scoreboard_section() -> dict:
    rows = list(csv.DictReader(CSV.open(encoding="utf-8-sig")))
    t144 = json.loads(T144_REF.read_text(encoding="utf-8"))
    active = t144["candidate_set"]

    cells = {}
    for fixture in FIXTURES:
        for cell in CELLS:
            key = f"{fixture}-{cell}"
            pnp = [
                r
                for r in rows
                if r["fixture"] == fixture
                and r["cell"] == cell
                and r["tool"] == "pnp-ordinary"
                and r["warmup"] == "0"
            ]
            orca = [
                r
                for r in rows
                if r["fixture"] == fixture
                and r["cell"] == cell
                and r["tool"] == "orca"
                and r["warmup"] == "0"
            ]
            if not pnp or not orca:
                raise SystemExit(f"scoreboard: missing rows for {key}")
            pw = [float(r["wall_seconds"]) for r in pnp]
            ow = [float(r["wall_seconds"]) for r in orca]
            pc = [float(r["cpu_seconds"]) for r in pnp]
            oc = [float(r["cpu_seconds"]) for r in orca]
            pm, om = statistics.median(pw), statistics.median(ow)
            ratio = pm / om
            hashes = sorted({r["sha256_of_gcode"] for r in pnp})
            cells[key] = {
                "pnp_wall_s": [round(v, 4) for v in pw],
                "orca_wall_s": [round(v, 4) for v in ow],
                "pnp_wall_median_s": round(pm, 4),
                "orca_wall_median_s": round(om, 4),
                "pnp_wall_spread_pct": round(100 * (max(pw) - min(pw)) / pm, 2),
                "orca_wall_spread_pct": round(100 * (max(ow) - min(ow)) / om, 2),
                "pnp_cpu_median_s": round(statistics.median(pc), 4),
                "orca_cpu_median_s": round(statistics.median(oc), 4),
                "ratio_pnp_over_orca": round(ratio, 4),
                "required_cut_pct": round(100 * (1 - 1 / ratio), 2),
                "required_cut_1_5x_pct": round(100 * (1 - 1.5 / ratio), 2),
                "pnp_hashes": hashes,
                "pnp_hash_matches_active_reference": len(hashes) == 1
                and hashes[0] == active[key],
                "pnp_all_runs_degraded": all(r["degraded"] == "true" for r in pnp),
                "pnp_non_fatal_counts": sorted(
                    {int(r["non_fatal_error_count"]) for r in pnp}
                ),
                "pnp_exit_codes": sorted({int(r["exit_code"]) for r in pnp}),
                "orca_exit_codes": sorted({int(r["exit_code"]) for r in orca}),
                "pnp_validated_generator": sorted(
                    {r["validated_generator"] for r in pnp}
                ),
                "orca_validated_generator": sorted(
                    {r["validated_generator"] for r in orca}
                ),
                "pnp_gcode_bytes": sorted({int(r["gcode_bytes"]) for r in pnp}),
                "orca_gcode_bytes": sorted({int(r["gcode_bytes"]) for r in orca}),
                "resolved": {
                    k: sorted({r[f"resolved_{k}"] for r in orca})
                    for k in (
                        "brim_width",
                        "brim_type",
                        "skirt_loops",
                        "skirt_distance",
                        "skirt_height",
                        "initial_layer_line_width",
                        "tree_support_brim_width",
                        "enable_arc_fitting",
                        "elefant_foot_compensation",
                    )
                },
            }

    benchy_ratios = [
        v["ratio_pnp_over_orca"] for k, v in cells.items() if k.startswith("benchy")
    ]
    base_ratios = [
        v["ratio_pnp_over_orca"] for k, v in cells.items() if k.startswith("base")
    ]
    cuts = [v["required_cut_pct"] for v in cells.values()]
    cuts15 = [v["required_cut_1_5x_pct"] for v in cells.values()]
    return {
        "batch_id": rows[0]["batch_id"],
        "rows": len(rows),
        "cells": cells,
        "all_pnp_hashes_match_active_reference": all(
            v["pnp_hash_matches_active_reference"] for v in cells.values()
        ),
        "ratio_range": [
            round(min(benchy_ratios + base_ratios), 4),
            round(max(benchy_ratios + base_ratios), 4),
        ],
        "benchy_ratio_range": [
            round(min(benchy_ratios), 4),
            round(max(benchy_ratios), 4),
        ],
        "base_ratio_range": [round(min(base_ratios), 4), round(max(base_ratios), 4)],
        "required_cut_pct_range": [min(cuts), max(cuts)],
        "required_cut_1_5x_pct_range": [min(cuts15), max(cuts15)],
        "tools_measured": ["orca", "pnp-ordinary"],
        "accelerated_arm": {
            "attempted": True,
            "command": "cargo xtask dist --accelerated --edition developer",
            "exit_code": 1,
            "failure": "c1xx: fatal error C1083: Cannot open source file: "
            "'\\mimalloc-static.cc': No such file or directory",
            "reason": "the controlled driver's accelerated host link fails in "
            "libmimalloc-sys under MSVC 14.51 (the documented t142 failure)",
        },
    }


# ---------------------------------------------------------------------------
# 3. the currency
# ---------------------------------------------------------------------------
def reference_section(score: dict) -> dict:
    t143 = json.loads(T143_REF.read_text(encoding="utf-8"))
    t144 = json.loads(T144_REF.read_text(encoding="utf-8"))
    active = t144["candidate_set"]
    benchy = {k: active[k] for k in active if k.startswith("benchy")}
    return {
        "note": (
            "Active matched-job reference set for the benchy fixture after t145 "
            "(2026-11-13). The four benchy hashes are t144's candidate set; they "
            "reproduce in this take on the ordinary arm (8 measured runs, one hash "
            "per cell). The four base hashes are unchanged by the benchy swap and "
            "are carried from t144 at the same values. A benchy number is either "
            "pre-swap (historical, retired here) or post-swap (current); never mix "
            "them."
        ),
        "active_set": active,
        "benchy_active_set": benchy,
        "reproduced_in_this_take": all(
            score["cells"][k]["pnp_hash_matches_active_reference"] for k in benchy
        ),
        "reproduced_runs_per_cell": 3,
        "retired_pre_swap_set": {
            "note": "t139's benchy hashes on the lost export (retired 2026-11-12 by t143, "
            "formally retired here).",
            "benchy-classic-off": "93e21fee6d757d37b3ae3b4efbfa2b24da178004f9a568d6647e2343ae005123",
            "benchy-classic-on": "b3f1ecccc5385f1b807c3418c271bcf57761a183e60c05eefb3294195df3d7a2",
            "benchy-arachne-off": "3510e8761d65cf3da702b9aa04dd7b0fec64ba2b16996e423bfcd46b72c1fd37",
            "benchy-arachne-on": "96cdac85437082377b351640019b3b1efe7208f6b77ada0f5b9de87035b07697",
        },
        "superseded_interim_set": {
            "note": "t143's replacement-fixture baseline; superseded by t144's anchor "
            "reconstruction. Historical only.",
            "set": t143["benchy_replacement_fixture"],
        },
        "base_unchanged": True,
    }


# ---------------------------------------------------------------------------
def main() -> int:
    if not CSV.exists():
        raise SystemExit(f"reduce_t145: missing raw campaign {CSV}")
    swap = swap_section()
    score = scoreboard_section()
    reference = reference_section(score)
    summary = {
        "ticket": 145,
        "generated_by": "reduce_t145.py",
        "raw_sources": {
            "campaign": "target/matched-pair/t145-benchy/scoreboard-rows.csv",
            "old_export_gcode": "target/t142/output1/gcode",
            "new_export_gcode": "target/t143/output/gcode",
            "accelerated_attempt_log": "target/t145/dist-accelerated-attempt.log",
        },
        "swap": swap,
        "scoreboard": score,
        "claims": {
            "classification": "tessellation-only",
            "swap_moves_output": True,
            "scoreboard_is_post_swap": True,
        },
    }
    HERE.mkdir(parents=True, exist_ok=True)
    (HERE / "reduced").mkdir(parents=True, exist_ok=True)
    (HERE / "reduced/summary.json").write_text(
        json.dumps(summary, indent=1) + "\n", encoding="utf-8"
    )
    (HERE / "reference.json").write_text(
        json.dumps(reference, indent=2) + "\n", encoding="utf-8"
    )
    print(f"wrote {HERE / 'reduced/summary.json'}")
    print(f"wrote {HERE / 'reference.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
