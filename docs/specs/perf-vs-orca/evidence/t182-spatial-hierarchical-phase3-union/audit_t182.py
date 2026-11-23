#!/usr/bin/env python3
"""t182 delta audit: byte/line comparison and per-section extruding path lengths
for both the base-classic-on parameter sweep (`--sweep`) and the full 8-cell
output campaign (`target/t182/output/ref/`).

Usage:
  python audit_t182.py --sweep        # audit round-1 sweep G-codes (k in {4, 8, 16})
  python audit_t182.py                # audit all 8 cells (baseline vs candidate ordinary)
  python audit_t182.py --self-test    # negative-control self-test on reduced/audit.json
"""

import copy
import hashlib
import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
GCODE_OUT = os.path.join(REPO, "target", "t182", "output", "ref")
GCODE_SWEEP = os.path.join(REPO, "target", "t182", "sweep", "sweep1")
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
SUPPORTS_OFF = [c for c in CELLS if c.endswith("-off")]
ALLOWED_SUPPORT_SECTIONS = {"Support", "Support interface", "Support transition"}

MOVE = re.compile(r"^G1\s+.*?X(-?[0-9.]+)\s+Y(-?[0-9.]+)")
XY_COORD = re.compile(r"X(-?[0-9.]+)\s+Y(-?[0-9.]+)")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_gcode_sections(path):
    """Return (paths_dict, line_count, non_support_motion_hash_no_e, max_non_support_de)
    where paths_dict maps (layer, section) -> extruding length in mm, and
    we also capture every non-support G1 motion line with E stripped for exact
    part-geometry invariance checking."""
    paths = {}
    layer = 0
    section = "?"
    x = y = None
    n_lines = 0
    part_motion_hasher = hashlib.sha256()
    part_motion_lines = 0
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            n_lines += 1
            if line.startswith(";LAYER_CHANGE"):
                layer += 1
                continue
            if line.startswith(";TYPE:"):
                section = line.strip().split(":", 1)[1]
                continue
            if not line.startswith("G1"):
                continue
            if section not in ALLOWED_SUPPORT_SECTIONS:
                stripped = re.sub(r"\s+E-?[0-9.]+", "", line.rstrip("\r\n"))
                part_motion_hasher.update(f"{layer}|{section}|{stripped}\n".encode("utf-8"))
                part_motion_lines += 1
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
    return paths, n_lines, part_motion_hasher.hexdigest(), part_motion_lines


def compare_part_geometry_lines(base_path, cand_path):
    """Verify exact motion-line invariance outside Support / Support interface
    (ignoring cumulative E drift, and tracking max |dE| on those lines)."""
    def iter_non_support_moves(path):
        layer = 0
        section = "?"
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for lineno, line in enumerate(fh, 1):
                if line.startswith(";LAYER_CHANGE"):
                    layer += 1
                elif line.startswith(";TYPE:"):
                    section = line.strip().split(":", 1)[1]
                elif line.startswith("G1") and section not in ALLOWED_SUPPORT_SECTIONS:
                    e_m = re.search(r"\sE(-?[0-9.]+)", line)
                    e_val = float(e_m.group(1)) if e_m else None
                    no_e = re.sub(r"\s+E-?[0-9.]+", "", line.rstrip("\r\n"))
                    yield (layer, section, no_e, e_val, lineno)

    it_b = iter_non_support_moves(base_path)
    it_c = iter_non_support_moves(cand_path)
    checked = 0
    mismatched_xy = 0
    max_de = 0.0
    examples = []
    while True:
        nb = next(it_b, None)
        nc = next(it_c, None)
        if nb is None and nc is None:
            break
        checked += 1
        if nb is None or nc is None or nb[:3] != nc[:3]:
            mismatched_xy += 1
            if len(examples) < 10:
                examples.append({"baseline": nb, "candidate": nc})
            if nb is None or nc is None:
                break
            continue
        if nb[3] is not None and nc[3] is not None:
            de = abs(nc[3] - nb[3])
            if de > max_de:
                max_de = de
    return {
        "non_support_g1_lines_checked": checked,
        "non_support_xy_mismatches": mismatched_xy,
        "non_support_max_abs_de_mm": round(max_de, 6),
        "examples": examples,
    }


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


def compare_pair(base_path, cand_path):
    if not (os.path.exists(base_path) and os.path.exists(cand_path)):
        return None
    b_sha = sha256_file(base_path)
    c_sha = sha256_file(cand_path)
    b_bytes = os.path.getsize(base_path)
    c_bytes = os.path.getsize(cand_path)
    byte_identical = b_sha == c_sha

    bp, b_lines, b_part_sha, b_part_n = parse_gcode_sections(base_path)
    if byte_identical:
        cp, c_lines, c_part_sha, c_part_n = bp, b_lines, b_part_sha, b_part_n
        part_audit = {
            "non_support_g1_lines_checked": b_part_n,
            "non_support_xy_mismatches": 0,
            "non_support_max_abs_de_mm": 0.0,
            "examples": [],
        }
    else:
        cp, c_lines, c_part_sha, c_part_n = parse_gcode_sections(cand_path)
        part_audit = compare_part_geometry_lines(base_path, cand_path)

    sections = sorted({k[1] for k in bp} | {k[1] for k in cp})
    got = {section: section_rows(bp, cp, section) for section in sections}
    moved_sections = [
        sec for sec, s in got.items() if abs(s["delta_mm"]) > 1e-9
    ]
    part_sections_moved = [
        sec for sec in moved_sections if sec not in ALLOWED_SUPPORT_SECTIONS
    ]
    b_total = sum(bp.values())
    c_total = sum(cp.values())
    return {
        "byte_identical": byte_identical,
        "baseline_sha256": b_sha,
        "candidate_sha256": c_sha,
        "baseline_bytes": b_bytes,
        "candidate_bytes": c_bytes,
        "delta_bytes": c_bytes - b_bytes,
        "baseline_lines": b_lines,
        "candidate_lines": c_lines,
        "delta_lines": c_lines - b_lines,
        "part_motion_sha256_equal": (b_part_sha == c_part_sha and b_part_n == c_part_n),
        "part_geometry_audit": part_audit,
        "moved_sections": moved_sections,
        "part_sections_moved": part_sections_moved,
        "sections": got,
        "total_extruding_mm": {
            "baseline": b_total,
            "candidate": c_total,
            "delta_mm": c_total - b_total,
            "delta_pct": (c_total - b_total) / b_total * 100.0,
        },
    }


def audit_sweep():
    base_path = os.path.join(GCODE_SWEEP, "r1__baseline.gcode")
    arms = ["worker-k4", "worker-k8", "worker-k16", "nested-k4", "nested-k8", "nested-k16"]
    out = {"schema": 1, "cell": "base-classic-on", "arms": {}}
    print(
        "%-12s %8s %10s %10s %10s %8s  %s"
        % ("arm", "d_bytes", "d_lines", "d_mm", "d_pct", "part_ok", "moved_sections")
    )
    for arm in arms:
        cand_path = os.path.join(GCODE_SWEEP, f"r1__{arm}.gcode")
        got = compare_pair(base_path, cand_path)
        if got is None:
            raise SystemExit(f"missing sweep gcode for {arm}: {cand_path}")
        out["arms"][arm] = got
        t = got["total_extruding_mm"]
        part_ok = (
            got["part_motion_sha256_equal"]
            and got["part_geometry_audit"]["non_support_xy_mismatches"] == 0
            and len(got["part_sections_moved"]) == 0
        )
        moved = ", ".join(
            f"{s} {got['sections'][s]['delta_mm']:+.4f}mm ({got['sections'][s]['delta_pct_of_section']:+.5f}%)"
            for s in got["moved_sections"]
        ) or "none"
        print(
            "%-12s %+8d %+10d %+10.5f %+9.6f%% %8s  %s"
            % (arm, got["delta_bytes"], got["delta_lines"], t["delta_mm"], t["delta_pct"], part_ok, moved)
        )
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "sweep_audit.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(out, fh, indent=2)
        fh.write("\n")
    print("wrote", path)
    return 0


def build():
    doc = {"schema": 1, "cells": {}}
    for cell in CELLS:
        base_path = os.path.join(GCODE_OUT, cell + "__baseline-ordinary.gcode")
        cand_path = os.path.join(GCODE_OUT, cell + "__candidate-ordinary.gcode")
        got = compare_pair(base_path, cand_path)
        if got:
            doc["cells"][cell] = got
    return doc


def check(doc, failures):
    cells = doc["cells"]
    if len(cells) != 8:
        failures.append("expected 8 cells, got %d" % len(cells))
    for cell, got in cells.items():
        if got["delta_bytes"] != got["candidate_bytes"] - got["baseline_bytes"]:
            failures.append("%s delta_bytes does not reconcile" % cell)
        if got["delta_lines"] != got["candidate_lines"] - got["baseline_lines"]:
            failures.append("%s delta_lines does not reconcile" % cell)
        if got["byte_identical"]:
            if got["baseline_sha256"] != got["candidate_sha256"]:
                failures.append("%s marked byte_identical but sha differs" % cell)
            if abs(got["total_extruding_mm"]["delta_mm"]) > 1e-12:
                failures.append("%s byte_identical but extruding delta != 0" % cell)
        else:
            if got["baseline_sha256"] == got["candidate_sha256"]:
                failures.append("%s marked non-identical but sha matches" % cell)
        if cell in SUPPORTS_OFF and not got["byte_identical"]:
            failures.append("supports-off cell %s is NOT byte_identical" % cell)
        if not got["part_motion_sha256_equal"]:
            failures.append("%s part_motion_sha256_equal is False" % cell)
        pa = got["part_geometry_audit"]
        if pa["non_support_xy_mismatches"] != 0:
            failures.append("%s has %d non-support XY mismatches" % (cell, pa["non_support_xy_mismatches"]))
        if pa["non_support_g1_lines_checked"] <= 0:
            failures.append("%s checked 0 non-support G1 lines" % cell)
        if got["part_sections_moved"]:
            failures.append("%s moved part sections: %s" % (cell, got["part_sections_moved"]))
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
    run(
        "supports-off cell moved",
        lambda d: d["cells"]["base-classic-off"].update(byte_identical=False, candidate_sha256="0" * 64),
    )
    run(
        "part geometry XY mismatch",
        lambda d: d["cells"]["base-classic-on"]["part_geometry_audit"].update(non_support_xy_mismatches=1),
    )
    run(
        "part section moved",
        lambda d: d["cells"]["base-classic-on"].update(part_sections_moved=["Outer wall"]),
    )
    return controls


def main(argv):
    if "--sweep" in argv:
        return audit_sweep()

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
                "%-36s %s %s"
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
        "%-22s %6s %8s %8s %10s %10s %7s  %s"
        % ("cell", "ident", "d_bytes", "d_lines", "total mm", "delta mm", "part_ok", "moved sections")
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
        )[:4]
        detail = (
            ", ".join(
                "%s %+.4f mm (%+.5f%%)" % (name, d, pct if pct is not None else 0.0)
                for _, name, d, pct in moved
                if abs(d) > 1e-12
            )
            or "no section moves"
        )
        part_ok = (
            got["part_motion_sha256_equal"]
            and got["part_geometry_audit"]["non_support_xy_mismatches"] == 0
            and not got["part_sections_moved"]
        )
        print(
            "%-22s %6s %+8d %+8d %10.2f %+10.5f %7s  %s"
            % (
                cell,
                "YES" if got["byte_identical"] else "NO",
                got["delta_bytes"],
                got["delta_lines"],
                got["total_extruding_mm"]["baseline"],
                got["total_extruding_mm"]["delta_mm"],
                "YES" if part_ok else "NO",
                detail,
            )
        )
    print("wrote", path)
    for f in failures:
        print("FAIL", f)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
