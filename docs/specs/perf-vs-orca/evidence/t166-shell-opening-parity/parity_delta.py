#!/usr/bin/env python3
"""t166 parity delta: per-layer solid-surface path, baseline vs candidate.

The opening decides **which** top/bottom surfaces survive Pass 1, so the parity
question is whether the candidate classifies *more* top/bottom surface than the
baseline on every layer (canonical's smaller radius deletes less), not whether
the totals happen to match.

Path length is the measurable proxy: both arms print at the same extrusion
width, so a layer's `Top surface` / `Bottom surface` path length is proportional
to its solid area. Every number here is re-derivable from the campaign's G-code
files alone.

Usage:  python parity_delta.py [--self-test]
Writes: reduced/parity.json
"""

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
GCODE = os.path.join(REPO, "target", "t166", "output", "ref")
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

# Sections whose printed path length stands in for solid *area*.
SOLID_SECTIONS = ("Top surface", "Bottom surface", "Internal solid infill", "Internal Bridge")

MOVE = re.compile(r"^G1\s+.*?X(-?[0-9.]+)\s+Y(-?[0-9.]+)")


def per_layer_path(path):
    """{(layer_index, section): millimetres}, plus the section totals."""
    out = {}
    totals = {}
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
                x = y = None  # a G1 that does not move XY breaks the chain
                continue
            nx, ny = float(m.group(1)), float(m.group(2))
            if x is not None and " E" in line:
                # Only extruding moves deposit material.
                d = ((nx - x) ** 2 + (ny - y) ** 2) ** 0.5
                key = (layer, section)
                out[key] = out.get(key, 0.0) + d
                totals[section] = totals.get(section, 0.0) + d
            x, y = nx, ny
    return out, totals


def compare_cell(cell):
    base_path = os.path.join(GCODE, cell + "__baseline-ordinary.gcode")
    cand_path = os.path.join(GCODE, cell + "__candidate-ordinary.gcode")
    if not (os.path.exists(base_path) and os.path.exists(cand_path)):
        return None
    base, base_totals = per_layer_path(base_path)
    cand, cand_totals = per_layer_path(cand_path)

    sections = {}
    for section in SOLID_SECTIONS:
        layers = sorted(
            {k[0] for k in base if k[1] == section}
            | {k[0] for k in cand if k[1] == section}
        )
        rows = []
        for layer in layers:
            b = base.get((layer, section), 0.0)
            c = cand.get((layer, section), 0.0)
            rows.append(
                {"layer": layer, "baseline_mm": b, "candidate_mm": c, "delta_mm": c - b}
            )
        more = sum(1 for r in rows if r["delta_mm"] > 1e-9)
        less = sum(1 for r in rows if r["delta_mm"] < -1e-9)
        equal = len(rows) - more - less
        worst = min(rows, key=lambda r: r["delta_mm"]) if rows else None
        sections[section] = {
            "baseline_mm": base_totals.get(section, 0.0),
            "candidate_mm": cand_totals.get(section, 0.0),
            "delta_mm": cand_totals.get(section, 0.0) - base_totals.get(section, 0.0),
            "delta_pct": (
                (cand_totals.get(section, 0.0) - base_totals.get(section, 0.0))
                / base_totals[section]
                * 100.0
                if base_totals.get(section)
                else None
            ),
            "layers_compared": len(rows),
            "layers_more": more,
            "layers_less": less,
            "layers_equal": equal,
            "largest_decrease": worst,
        }

    # First layer at which any solid section differs, and where.
    first = None
    for layer in sorted({k[0] for k in base} | {k[0] for k in cand}):
        for section in SOLID_SECTIONS:
            b = base.get((layer, section), 0.0)
            c = cand.get((layer, section), 0.0)
            if abs(c - b) > 1e-9:
                first = {
                    "layer": layer,
                    "section": section,
                    "baseline_mm": b,
                    "candidate_mm": c,
                }
                break
        if first:
            break

    return {"sections": sections, "first_differing_layer": first}


def build():
    doc = {"schema": 1, "cells": {}}
    for cell in CELLS:
        got = compare_cell(cell)
        if got:
            doc["cells"][cell] = got
    return doc


def check(doc, failures):
    """Independent assertions over the reduced document."""
    cells = doc["cells"]
    if len(cells) != 8:
        failures.append("expected 8 cells, got %d" % len(cells))
    for cell, got in cells.items():
        for section in ("Top surface", "Bottom surface"):
            s = got["sections"][section]
            if s["delta_mm"] <= 0.0:
                failures.append(
                    "%s %s did not grow (%.3f mm)" % (cell, section, s["delta_mm"])
                )
            # Totals must reconcile with the per-layer rows.
            if (
                abs(
                    s["layers_more"]
                    + s["layers_less"]
                    + s["layers_equal"]
                    - s["layers_compared"]
                )
                > 0
            ):
                failures.append("%s %s layer census does not sum" % (cell, section))
            if s["delta_pct"] is None:
                failures.append(
                    "%s %s has no baseline to compare against" % (cell, section)
                )
    # Only the two sections the opening classifies carry a direction claim;
    # the internal sections are reported for the delta audit, not gated (the
    # baseline prints 8 lines of `Internal solid infill` on one base cell and
    # zero elsewhere, so "non-empty" is not a meaningful gate).
    for section in ("Top surface", "Bottom surface"):
        if not any(cells[c]["sections"][section]["baseline_mm"] > 0 for c in cells):
            failures.append("section %s is empty in every cell" % section)


def self_test(doc):
    """Negative controls: each mutation must trip a check."""
    import copy

    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run(
        "top-surface total zeroed",
        lambda d: d["cells"]["base-classic-off"]["sections"]["Top surface"].update(
            delta_mm=0.0
        ),
    )
    run(
        "bottom-surface shrunk",
        lambda d: d["cells"]["benchy-arachne-off"]["sections"]["Bottom surface"].update(
            delta_mm=-1.0
        ),
    )
    run(
        "layer census broken",
        lambda d: d["cells"]["base-arachne-on"]["sections"]["Top surface"].update(
            layers_more=1_000_000
        ),
    )
    run("a cell dropped", lambda d: d["cells"].pop("benchy-classic-off"))
    run(
        "no baseline for a section",
        lambda d: d["cells"]["base-classic-on"]["sections"]["Bottom surface"].update(
            delta_pct=None
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
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "parity.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")

    failures = []
    check(doc, failures)
    for cell in CELLS:
        if cell not in doc["cells"]:
            continue
        s = doc["cells"][cell]["sections"]
        print(
            "%-22s top %+8.1f mm (%3d more / %3d less)  bottom %+8.1f mm (%3d more / %3d less)"
            % (
                cell,
                s["Top surface"]["delta_mm"],
                s["Top surface"]["layers_more"],
                s["Top surface"]["layers_less"],
                s["Bottom surface"]["delta_mm"],
                s["Bottom surface"]["layers_more"],
                s["Bottom surface"]["layers_less"],
            )
        )
    print("wrote", path)
    if failures:
        for f in failures:
            print("FAIL", f)
        return 1
    print("PASS: top and bottom solid path grows in all eight cells (per-layer redistribution is reported above, not gated)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
