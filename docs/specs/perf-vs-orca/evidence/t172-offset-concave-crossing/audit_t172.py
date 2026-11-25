#!/usr/bin/env python3
"""t172 delta audit: which G-code sections move, by how much, and on how many layers.

The candidate ports canonical `v2.5.0`'s `clipper.offset.cpp::OffsetConcaveCrossing`
into `vendor/clipper2-rust/src/offset.rs`, which joins concave corners directly at
the crossing of both edge offsets instead of emitting 3-point concave loops for the
finishing union to remove. Output moves because the crossing point is rounded in
one step rather than clipped from a 3-point loop.

Usage:  python audit_t172.py [--self-test]
Writes: reduced/audit.json
"""

import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
GCODE = os.path.join(REPO, "target", "t172", "output", "ref")
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

MOVE = re.compile(r"^G1\s+.*?X(-?[0-9.]+)\s+Y(-?[0-9.]+)")


def per_layer(path):
    """{((layer, section)): extruding length in mm}."""
    paths = {}
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
                d = math.hypot(nx - x, ny - y)
                if d > 0.0:
                    key = (layer, section)
                    paths[key] = paths.get(key, 0.0) + d
            x, y = nx, ny
    return paths


def section_rows(bp, cp, section):
    layers = sorted(
        {k[0] for k in bp if k[1] == section} | {k[0] for k in cp if k[1] == section}
    )
    rows = []
    for layer in layers:
        b = bp.get((layer, section), 0.0)
        c = cp.get((layer, section), 0.0)
        rows.append({"layer": layer, "baseline_mm": b, "candidate_mm": c})
    b_total = sum(r["baseline_mm"] for r in rows)
    c_total = sum(r["candidate_mm"] for r in rows)
    return {
        "baseline_mm": b_total,
        "candidate_mm": c_total,
        "delta_mm": c_total - b_total,
        "delta_pct_of_section": ((c_total - b_total) / b_total * 100.0)
        if b_total > 0.0
        else None,
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


def compare_cell(cell):
    base_path = os.path.join(GCODE, cell + "__baseline-ordinary.gcode")
    cand_path = os.path.join(GCODE, cell + "__candidate-ordinary.gcode")
    if not (os.path.exists(base_path) and os.path.exists(cand_path)):
        return None
    bp = per_layer(base_path)
    cp = per_layer(cand_path)
    sections = sorted({k[1] for k in bp} | {k[1] for k in cp})
    got = {section: section_rows(bp, cp, section) for section in sections}
    b_total = sum(bp.values())
    c_total = sum(cp.values())
    return {
        "sections": got,
        "total_extruding_mm": {
            "baseline": b_total,
            "candidate": c_total,
            "delta_mm": c_total - b_total,
            "delta_pct": (c_total - b_total) / b_total * 100.0,
        },
    }


def build():
    doc = {"schema": 1, "cells": {}}
    for cell in CELLS:
        got = compare_cell(cell)
        if got:
            doc["cells"][cell] = got
    return doc


def check(doc, failures):
    cells = doc["cells"]
    if len(cells) != 8:
        failures.append("expected 8 cells, got %d" % len(cells))
    for cell, got in cells.items():
        for section, s in got["sections"].items():
            if (
                s["layers_more"] + s["layers_less"] + s["layers_equal"]
                != s["layers_compared"]
            ):
                failures.append("%s %s layer census does not sum" % (cell, section))
            if abs(s["baseline_mm"] - s["candidate_mm"] - (-s["delta_mm"])) > 1e-6:
                failures.append("%s %s delta does not reconcile" % (cell, section))
            if s["layers_compared"] == 0 and abs(s["delta_mm"]) > 0.0:
                failures.append("%s %s delta without layers" % (cell, section))
        t = got["total_extruding_mm"]
        if abs(t["baseline"] - t["candidate"] - (-t["delta_mm"])) > 1e-6:
            failures.append("%s total path delta does not reconcile" % cell)
        summed = sum(s["delta_mm"] for s in got["sections"].values())
        if abs(summed - t["delta_mm"]) > 1e-6:
            failures.append("%s section deltas do not sum to the total" % cell)


def self_test(doc):
    import copy

    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    any_cell = sorted(doc["cells"])[0]
    first_section = sorted(doc["cells"][any_cell]["sections"])[0]
    run(
        "census broken",
        lambda d: d["cells"][any_cell]["sections"][first_section].update(
            layers_more=10**6
        ),
    )
    run(
        "delta does not reconcile",
        lambda d: d["cells"][any_cell]["sections"][first_section].update(delta_mm=1.0),
    )
    run("cell dropped", lambda d: d["cells"].pop(sorted(d["cells"])[-1]))
    run(
        "section deltas do not sum",
        lambda d: d["cells"][any_cell]["total_extruding_mm"].update(delta_mm=999.0),
    )
    return controls


def main(argv):
    if "--self-test" in argv:
        path = os.path.join(OUT, "audit.json")
        if os.path.exists(path):
            with open(path, encoding="utf-8") as fh:
                doc = json.load(fh)
        else:
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
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "audit.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")

    failures = []
    check(doc, failures)
    print(
        "%-22s %10s %10s  %s"
        % ("cell", "total mm", "delta mm", "largest section deltas")
    )
    for cell in CELLS:
        if cell not in doc["cells"]:
            continue
        got = doc["cells"][cell]
        moved = sorted(
            (
                (abs(s["delta_mm"]), name, s["delta_mm"], s["delta_pct_of_section"])
                for name, s in got["sections"].items()
            ),
            reverse=True,
        )[:3]
        detail = (
            ", ".join(
                "%s %+.2f mm (%+.3f%%)" % (name, d, pct if pct is not None else 0.0)
                for _, name, d, pct in moved
                if d != 0.0
            )
            or "no section moves"
        )
        print(
            "%-22s %10.2f %+10.2f  %s"
            % (
                cell,
                got["total_extruding_mm"]["baseline"],
                got["total_extruding_mm"]["delta_mm"],
                detail,
            )
        )
    print("wrote", path)
    for f in failures:
        print("FAIL", f)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
