#!/usr/bin/env python3
"""t168 delta audit: bridge / internal-bridge / sparse / gap section deltas.

The candidate resolves canonical's auto sentinel for the *bridge* opening
(`outer_wall_line_width = 0` -> `line_width`), so the bridge classifier deletes
the sub-printable sliver tail it used to keep. The parity question is
directional, not numeric: the expected direction is **fewer** `Bridge` lines
and **more** `Sparse infill`, never the reverse. This tool measures that
direction per cell and per layer, and reports the reverse rather than hiding it.

Bridge-anchor direction cannot be read from G-code as `bridge_orientation_deg`
(the IR field is not printed), so it is measured here as the length-weighted
dominant angle of the `Bridge` section's extruding segments, in degrees mod 180
(doubled-angle circular mean). It is a proxy, labelled as one: a real direction
flip between the arms shows up as an angle delta near 90 degrees.

Usage:  python audit_t168.py [--self-test]
Writes: reduced/audit.json
"""

import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
GCODE = os.path.join(REPO, "target", "t168", "output", "ref")
OUT = os.path.join(HERE, "reduced")

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

AUDIT_SECTIONS = ("Bridge", "Internal Bridge", "Sparse infill", "Gap infill")

MOVE = re.compile(r"^G1\s+.*?X(-?[0-9.]+)\s+Y(-?[0-9.]+)")


def per_layer(path):
    """(path_mm, direction_vector) per (layer, section).

    `direction_vector` sums unit-length segment contributions doubled in angle,
    so the dominant orientation recovers as atan2(vy, vx) / 2.
    """
    paths = {}
    dirs = {}
    layer = 0
    section = "?"
    x = y = None
    with open(path, "r", errors="replace") as fh:
        for line in fh:
            if line.startswith(";LAYER_CHANGE"):
                layer += 1
                continue
            if line.startswith(";TYPE:"):
                section = line.strip().split(":", 1)[1]
                continue
            if not line.startswith("G1"):
                continue
            m = MOVE.match(line)
            if not m:
                x = y = None
                continue
            nx, ny = float(m.group(1)), float(m.group(2))
            if x is not None and " E" in line:
                dx, dy = nx - x, ny - y
                d = math.hypot(dx, dy)
                if d > 0.0:
                    key = (layer, section)
                    paths[key] = paths.get(key, 0.0) + d
                    theta2 = 2.0 * math.atan2(dy, dx)
                    vx, vy = dirs.get(key, (0.0, 0.0))
                    dirs[key] = (vx + d * math.cos(theta2), vy + d * math.sin(theta2))
            x, y = nx, ny
    return paths, dirs


def dominant_deg(vec):
    if vec is None:
        return None
    vx, vy = vec
    if abs(vx) < 1e-12 and abs(vy) < 1e-12:
        return None
    return (math.degrees(math.atan2(vy, vx)) / 2.0) % 180.0


def angle_delta(a, b):
    if a is None or b is None:
        return None
    d = abs(a - b) % 180.0
    return min(d, 180.0 - d)


def totals_by_section(paths):
    out = {}
    for (_, section), mm in paths.items():
        out[section] = out.get(section, 0.0) + mm
    return out


def compare_cell(cell):
    base_path = os.path.join(GCODE, cell + "__baseline-ordinary.gcode")
    cand_path = os.path.join(GCODE, cell + "__candidate-ordinary.gcode")
    if not (os.path.exists(base_path) and os.path.exists(cand_path)):
        return None
    bp, bd = per_layer(base_path)
    cp, cd = per_layer(cand_path)

    sections = {}
    for section in AUDIT_SECTIONS:
        layers = sorted(
            {k[0] for k in bp if k[1] == section}
            | {k[0] for k in cp if k[1] == section}
        )
        rows = []
        for layer in layers:
            b = bp.get((layer, section), 0.0)
            c = cp.get((layer, section), 0.0)
            rows.append({"layer": layer, "baseline_mm": b, "candidate_mm": c})
        b_total = sum(r["baseline_mm"] for r in rows)
        c_total = sum(r["candidate_mm"] for r in rows)
        sections[section] = {
            "baseline_mm": b_total,
            "candidate_mm": c_total,
            "delta_mm": c_total - b_total,
            "layers_compared": len(rows),
            "layers_more": sum(
                1 for r in rows if r["candidate_mm"] - r["baseline_mm"] > 1e-9
            ),
            "layers_less": sum(
                1 for r in rows if r["candidate_mm"] - r["baseline_mm"] < -1e-9
            ),
            "layers_equal": sum(
                1 for r in rows if abs(r["candidate_mm"] - r["baseline_mm"]) <= 1e-9
            ),
            "first_differing_layer": next(
                (
                    r["layer"]
                    for r in rows
                    if abs(r["candidate_mm"] - r["baseline_mm"]) > 1e-9
                ),
                None,
            ),
        }

    # Bridge-anchor direction proxy, per layer that prints a Bridge section.
    bridge_layers = sorted(
        {k[0] for k in bp if k[1] == "Bridge"} | {k[0] for k in cp if k[1] == "Bridge"}
    )
    direction = []
    for layer in bridge_layers:
        b = dominant_deg(bd.get((layer, "Bridge")))
        c = dominant_deg(cd.get((layer, "Bridge")))
        direction.append(
            {
                "layer": layer,
                "baseline_deg": b,
                "candidate_deg": c,
                "delta_deg": angle_delta(b, c),
            }
        )
    flips = [
        d for d in direction if d["delta_deg"] is not None and d["delta_deg"] >= 45.0
    ]

    return {
        "sections": sections,
        "bridge_direction": {
            "layers": len(direction),
            "flips_ge_45deg": len(flips),
            "flip_layers": [d["layer"] for d in flips],
            "per_layer": direction,
        },
    }


def build():
    doc = {"schema": 1, "cells": {}}
    for cell in CELLS:
        got = compare_cell(cell)
        if got:
            doc["cells"][cell] = got
    return doc


def direction_verdict(cell, got):
    """Classify the measured direction for one cell (report, never a gate)."""
    bridge = got["sections"]["Bridge"]["delta_mm"]
    sparse = got["sections"]["Sparse infill"]["delta_mm"]
    if bridge < 0.0 and sparse > 0.0:
        return "expected (fewer bridge, more sparse)"
    if bridge > 0.0 and sparse < 0.0:
        return "REVERSE (more bridge, less sparse)"
    if abs(bridge) <= 1e-9 and abs(sparse) <= 1e-9:
        return "inert"
    return "mixed"


def check(doc, failures):
    cells = doc["cells"]
    if len(cells) != 8:
        failures.append("expected 8 cells, got %d" % len(cells))
    for cell, got in cells.items():
        for section in AUDIT_SECTIONS:
            s = got["sections"][section]
            if (
                s["layers_more"] + s["layers_less"] + s["layers_equal"]
                != s["layers_compared"]
            ):
                failures.append("%s %s layer census does not sum" % (cell, section))
            rows_total = s["layers_compared"]
            if (
                rows_total
                and abs(s["baseline_mm"] - s["candidate_mm"] - (-s["delta_mm"])) > 1e-6
            ):
                failures.append("%s %s delta does not reconcile" % (cell, section))
            if s["layers_compared"] == 0 and abs(s["delta_mm"]) > 0.0:
                failures.append("%s %s delta without layers" % (cell, section))
        bd = got["bridge_direction"]
        if bd["layers"] != len(bd["per_layer"]):
            failures.append("%s bridge layer census does not sum" % cell)
        if len(bd["flip_layers"]) != bd["flips_ge_45deg"]:
            failures.append("%s bridge flip list does not reconcile" % cell)


def self_test(doc):
    import copy

    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run(
        "census broken",
        lambda d: d["cells"]["base-classic-off"]["sections"]["Bridge"].update(
            layers_more=10**6
        ),
    )
    run(
        "delta does not reconcile",
        lambda d: d["cells"]["benchy-classic-off"]["sections"]["Sparse infill"].update(
            delta_mm=1.0
        ),
    )
    run("cell dropped", lambda d: d["cells"].pop("base-arachne-on"))
    run(
        "bridge layer census broken",
        lambda d: d["cells"]["base-classic-on"]["bridge_direction"].update(layers=0),
    )
    run(
        "flip list does not reconcile",
        lambda d: d["cells"]["base-classic-off"]["bridge_direction"].update(
            flip_layers=[]
        ),
    )
    return controls


def main(argv):
    if "--self-test" in argv:
        doc = build()
        controls = self_test(doc)
        ok = all(rejected for _, rejected, _ in controls)
        for name, rejected, detail in controls:
            print(
                "%-32s %s %s"
                % (
                    name,
                    "REJECTED" if rejected else "ACCEPTED",
                    detail if rejected else "",
                )
            )
        print("self-test:", "PASS" if ok else "FAIL")
        return 0 if ok else 1

    doc = build()
    for cell, got in doc["cells"].items():
        got["direction_verdict"] = direction_verdict(cell, got)
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "audit.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")

    failures = []
    check(doc, failures)
    print(
        "%-22s %14s %14s %14s %14s  %s"
        % ("cell", "Bridge", "InternalBridge", "Sparse", "Gap", "direction")
    )
    for cell in CELLS:
        if cell not in doc["cells"]:
            continue
        got = doc["cells"][cell]
        s = got["sections"]
        print(
            "%-22s %+14.1f %+14.1f %+14.1f %+14.1f  %s"
            % (
                cell,
                s["Bridge"]["delta_mm"],
                s["Internal Bridge"]["delta_mm"],
                s["Sparse infill"]["delta_mm"],
                s["Gap infill"]["delta_mm"],
                got["direction_verdict"],
            )
        )
    print("wrote", path)
    for f in failures:
        print("FAIL", f)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
