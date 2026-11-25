#!/usr/bin/env python3
"""t175 delta audit: byte/line comparison, per-section extruding path lengths,
and exhaustive Clipper2 scanbeam-clamping provenance.

Reads `target/t175/output/ref/<cell>__baseline-ordinary.gcode` vs
`<cell>__candidate-ordinary.gcode` for all 8 cells (plus the exhaustive
phase-1 call audit logs under `target/t175-sweep/` if present) and writes
`reduced/audit.json`.

Usage:  python audit_t175.py [--self-test]
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
GCODE = os.path.join(REPO, "target", "t175", "output", "ref")
SWEEP_DIAG = os.path.join(REPO, "target", "t175-sweep")
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
XY_COORD = re.compile(r"X(-?[0-9.]+)\s+Y(-?[0-9.]+)")
BATCH_RE = re.compile(
    r"AUDIT_BATCH empty=(\d+)/(\d+) covered=(\d+)/(\d+) polys=(\d+)/(\d+) explained=(\d+)"
)
MISMATCH_RE = re.compile(
    r"AUDIT_MISMATCH kind=(\d+) rb=\((-?\d+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)\) "
    r"flen=([^\s]+) wlen=([^\s]+) diff_v=(\d+) max_d=(-?\d+) scanline_explained=(true|false)"
)
FAR_Y_RE = re.compile(r"DIAG FAR_Y_(\d+): count=(\d+) samples=\[(.*)\]")
DUMMY_PROOF_RE = re.compile(
    r"DIAG DUMMY_SCANLINE_PROOF: full_eq_win=(true|false) full_eq_win_with_dummy_far_y=(true|false)"
)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def per_layer_and_lines(path):
    """Return (paths_dict, line_count, indexed_lines) where paths_dict maps
    (layer, section) -> extruding length in mm."""
    paths = {}
    layer = 0
    section = "?"
    x = y = None
    n_lines = 0
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
    return paths, n_lines


def line_diffs(base_path, cand_path, max_examples=50):
    """Line-by-line diff when file line counts match (or fallback prefix scan)."""
    diffs = []
    total_diffs = 0
    b_layer = 0
    b_section = "?"
    with (
        open(base_path, "r", encoding="utf-8", errors="replace") as fb,
        open(cand_path, "r", encoding="utf-8", errors="replace") as fc,
    ):
        lineno = 0
        while True:
            lb = fb.readline()
            lc = fc.readline()
            if not lb and not lc:
                break
            lineno += 1
            if lb.startswith(";LAYER_CHANGE"):
                b_layer += 1
            elif lb.startswith(";TYPE:"):
                b_section = lb.strip().split(":", 1)[1]
            if lb != lc:
                total_diffs += 1
                if len(diffs) < max_examples:
                    entry = {
                        "line": lineno,
                        "layer": b_layer,
                        "section": b_section,
                        "baseline": lb.rstrip("\r\n"),
                        "candidate": lc.rstrip("\r\n"),
                    }
                    mb = XY_COORD.search(entry["baseline"])
                    mc = XY_COORD.search(entry["candidate"])
                    if mb and mc:
                        dx_mm = float(mc.group(1)) - float(mb.group(1))
                        dy_mm = float(mc.group(2)) - float(mb.group(2))
                        entry["dx_mm"] = round(dx_mm, 6)
                        entry["dy_mm"] = round(dy_mm, 6)
                        entry["dist_um"] = round(math.hypot(dx_mm, dy_mm) * 1000.0, 4)
                    diffs.append(entry)
    return total_diffs, diffs


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
    b_sha = sha256_file(base_path)
    c_sha = sha256_file(cand_path)
    b_bytes = os.path.getsize(base_path)
    c_bytes = os.path.getsize(cand_path)
    byte_identical = b_sha == c_sha

    bp, b_lines = per_layer_and_lines(base_path)
    if byte_identical:
        cp, c_lines = bp, b_lines
        n_diff_lines, examples = 0, []
    else:
        cp, c_lines = per_layer_and_lines(cand_path)
        n_diff_lines, examples = line_diffs(base_path, cand_path)

    sections = sorted({k[1] for k in bp} | {k[1] for k in cp})
    got = {section: section_rows(bp, cp, section) for section in sections}
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
        "differing_lines": n_diff_lines,
        "diff_examples": examples,
        "sections": got,
        "total_extruding_mm": {
            "baseline": b_total,
            "candidate": c_total,
            "delta_mm": c_total - b_total,
            "delta_pct": (c_total - b_total) / b_total * 100.0,
        },
    }


def parse_scanbeam_audit():
    """Parse exhaustive per-call audit logs from target/t175-sweep/ if present."""
    result = {}
    for cell, fname in (
        ("base-classic-on", "audit_base-classic-on.err"),
        ("benchy-classic-on", "audit_benchy-classic-on.err"),
    ):
        path = os.path.join(SWEEP_DIAG, fname)
        if not os.path.exists(path):
            continue
        empty_mm = empty_tot = cov_mm = cov_tot = poly_mm = poly_tot = bulk_expl = 0
        batches = 0
        mismatches = []
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                mb = BATCH_RE.search(line)
                if mb:
                    batches += 1
                    empty_mm += int(mb.group(1))
                    empty_tot += int(mb.group(2))
                    cov_mm += int(mb.group(3))
                    cov_tot += int(mb.group(4))
                    poly_mm += int(mb.group(5))
                    poly_tot += int(mb.group(6))
                    bulk_expl += int(mb.group(7))
                    continue
                mm = MISMATCH_RE.search(line)
                if mm:
                    mismatches.append(
                        {
                            "kind": int(mm.group(1)),
                            "rb": [
                                int(mm.group(2)),
                                int(mm.group(3)),
                                int(mm.group(4)),
                                int(mm.group(5)),
                            ],
                            "flen": mm.group(6),
                            "wlen": mm.group(7),
                            "diff_v": int(mm.group(8)),
                            "max_d_units": int(mm.group(9)),
                            "bulk_scanline_explained": mm.group(10) == "true",
                        }
                    )
        result[cell] = {
            "batches": batches,
            "total_calls_audited": empty_tot + cov_tot + poly_tot,
            "empty_mismatches": empty_mm,
            "empty_calls": empty_tot,
            "covered_mismatches": cov_mm,
            "covered_calls": cov_tot,
            "polys_mismatches": poly_mm,
            "polys_calls": poly_tot,
            "bulk_dummy_scanline_explained": bulk_expl,
            "mismatches": mismatches,
        }

    proof_path = os.path.join(SWEEP_DIAG, "diag_proof.err")
    if os.path.exists(proof_path):
        far_y = None
        dummy_proof = None
        with open(proof_path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                mf = FAR_Y_RE.search(line)
                if mf:
                    far_y = {
                        "y_units": int(mf.group(1)),
                        "distant_vertex_count": int(mf.group(2)),
                        "samples": mf.group(3),
                    }
                mp = DUMMY_PROOF_RE.search(line)
                if mp:
                    dummy_proof = {
                        "target_rb": [1001681, 1053255, 1128598, 1180262],
                        "full_eq_win": mp.group(1) == "true",
                        "full_eq_win_with_dummy_far_y": mp.group(2) == "true",
                    }
        if far_y or dummy_proof:
            result["isolated_9th_call_proof"] = {
                "far_y_scanline": far_y,
                "dummy_scanline_proof": dummy_proof,
                "clipper2_site": "vendor/clipper2-rust/src/engine.rs::add_new_intersect_node",
                "mechanism": (
                    "Clipper2 populates global `scanline_list` with the Y coordinate of "
                    "every vertex in `clip`, including distant unrelated vertices 14-36 mm "
                    "away. `add_new_intersect_node` clamps shallow intersections to the "
                    "nearest scanbeam (`bot_y`/`top_y`). Omitting distant vertices removes "
                    "the spurious scanline, leaving the true unclamped intersection."
                ),
            }
    return result


def build():
    doc = {"schema": 1, "cells": {}, "scanbeam_audit": parse_scanbeam_audit()}
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
        if got["delta_bytes"] != got["candidate_bytes"] - got["baseline_bytes"]:
            failures.append("%s delta_bytes does not reconcile" % cell)
        if got["byte_identical"]:
            if got["baseline_sha256"] != got["candidate_sha256"]:
                failures.append("%s marked byte_identical but sha differs" % cell)
            if got["differing_lines"] != 0:
                failures.append("%s byte_identical but differing_lines != 0" % cell)
            if abs(got["total_extruding_mm"]["delta_mm"]) > 1e-12:
                failures.append("%s byte_identical but extruding delta != 0" % cell)
        else:
            if got["baseline_sha256"] == got["candidate_sha256"]:
                failures.append("%s marked non-identical but sha matches" % cell)
            if got["differing_lines"] <= 0:
                failures.append("%s non-identical but differing_lines <= 0" % cell)
            if len(got["diff_examples"]) == 0:
                failures.append("%s non-identical but diff_examples empty" % cell)
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

    sb = doc.get("scanbeam_audit") or {}
    base_sb = sb.get("base-classic-on")
    if base_sb:
        if (
            base_sb["empty_calls"] + base_sb["covered_calls"] + base_sb["polys_calls"]
            != base_sb["total_calls_audited"]
        ):
            failures.append("scanbeam_audit base-classic-on call counts do not sum")
        if base_sb["empty_mismatches"] != 0 or base_sb["covered_mismatches"] != 0:
            failures.append("scanbeam_audit base-classic-on empty/covered mismatch != 0")
        if len(base_sb["mismatches"]) != base_sb["polys_mismatches"]:
            failures.append("scanbeam_audit base-classic-on mismatch list length mismatch")
        iso = sb.get("isolated_9th_call_proof", {}).get("dummy_scanline_proof", {})
        if not (iso.get("full_eq_win") is False and iso.get("full_eq_win_with_dummy_far_y") is True):
            failures.append("scanbeam_audit isolated 9th call proof missing or negative")


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
        "byte_identical with differing lines",
        lambda d: d["cells"]["benchy-classic-on"].update(differing_lines=3),
    )
    run(
        "scanbeam empty mismatch non-zero",
        lambda d: d["scanbeam_audit"]["base-classic-on"].update(empty_mismatches=1),
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
        "%-22s %6s %8s %10s %10s  %s"
        % ("cell", "ident", "diff_ln", "total mm", "delta mm", "largest section deltas")
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
                "%s %+.4f mm (%+.5f%%)" % (name, d, pct if pct is not None else 0.0)
                for _, name, d, pct in moved
                if abs(d) > 1e-12
            )
            or "no section moves"
        )
        print(
            "%-22s %6s %8d %10.2f %+10.5f  %s"
            % (
                cell,
                "YES" if got["byte_identical"] else "NO",
                got["differing_lines"],
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
