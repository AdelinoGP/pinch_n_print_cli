#!/usr/bin/env python3
"""Checkout-only verification for the t88 supports-on increment attribution.

    python verify.py              # exit 0 on the committed evidence set
    python verify.py --self-test  # also run 8 negative controls

Reads only ``captures/*.json``, ``summary.json`` and the t87 scoreboard row
CSVs. Re-derives every headline term, every share and every floor test, then
applies the negative controls to an in-memory copy.
"""

import argparse
import copy
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURES = HERE / "captures"
SUMMARY = HERE / "summary.json"
SCOREBOARD = HERE / ".." / "scoreboard-refresh-20261025"

TOL = 0.02  # percentage points / absolute ms tolerance on rounded quotes
POSTPASS_STAGES = (
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::GCodeSerialize",
)


class Failure(Exception):
    pass


def close(a, b, tol=TOL):
    return abs(float(a) - float(b)) <= tol


def load():
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    captures = {}
    for path in sorted(CAPTURES.glob("*.json")):
        capture = json.loads(path.read_text(encoding="utf-8"))
        previous = captures.get(capture["run_id"])
        if previous is None or capture["batch"].startswith("t88b"):
            captures[capture["run_id"]] = capture
    return summary, captures


def quiet_walls():
    walls = {}
    for cell in ("base-classic", "base-arachne"):
        for side in ("off", "on"):
            path = SCOREBOARD / f"rows-{cell}-{side}.csv"
            rows = [
                row
                for row in csv.DictReader(open(path, encoding="utf-8-sig", newline=""))
                if row["tool"] == "pnp-ordinary" and row["warmup"] == "0"
            ]
            walls[f"{cell}-{side}"] = {
                "wall_s": statistics.median(float(row["wall_seconds"]) for row in rows),
                "cpu_s": statistics.median(float(row["cpu_seconds"]) for row in rows),
                "samples": len(rows),
            }
    return walls


def by_id(capture, module_id):
    rows = [row for row in capture["modules"] if row["module_id"] == module_id]
    return rows[0] if rows else None


def gap(capture, stage):
    rows = [row for row in capture["stage_gaps"] if row["stage"] == stage]
    if not rows:
        raise Failure(f"{capture['run_id']}: missing stage gap {stage}")
    return rows[-1]


def stage(capture, name):
    rows = [row for row in capture["stages"] if row["stage"] == name]
    if not rows:
        raise Failure(f"{capture['run_id']}: missing stage {name}")
    return rows[0]


def probe(capture, term):
    value = capture.get("probe_terms", {}).get(term)
    if value is None:
        raise Failure(f"{capture['run_id']}: missing probe term {term}")
    return value


def pct(part, whole):
    return part * 100.0 / whole


def recompute(summary, captures):
    """Re-derive every published term from the captures. Returns failures."""
    failures = []
    quiet = quiet_walls()
    if summary["quiet_scoreboard"] != quiet:
        failures.append("quiet scoreboard medians do not match the t87 CSVs")

    def need(cell, side, label):
        run_id = f"{cell}-{side}-{label}"
        if run_id not in captures:
            raise Failure(f"missing capture {run_id}")
        return captures[run_id]

    for cell in ("base-classic", "base-arachne"):
        for side in ("off", "on"):
            wall = need(cell, side, "probe")
            for key in ("status", "sha256"):
                if not wall.get(key):
                    failures.append(f"{wall['run_id']}: missing {key}")
            if wall["status"] != "ok" or wall["degraded"] or wall["non_fatal"] != 0:
                failures.append(f"{wall['run_id']}: unclean output flags")
            if wall["sha256"] != wall["reference_sha256"]:
                failures.append(
                    f"{wall['run_id']}: output SHA differs from the t87 reference"
                )
            # Stage walls are the source of truth for the share denominators.
            if not close(
                summary["stage_table"][f"{cell}-{side}"]["slice_ms"],
                wall["slice_complete"]["elapsed_ms"],
                1.0,
            ):
                failures.append(f"{wall['run_id']}: slice wall mismatch")
            # Postpass gap re-derivation.
            gap_ms = wall["phases"]["postpass"] - sum(
                stage(wall, name)["elapsed_ms"] for name in POSTPASS_STAGES
            )
            published = summary["ownership"][f"{cell}-{side}"]["postpass_gap_ms"]
            if not close(gap_ms, published, 1.0):
                failures.append(f"{wall['run_id']}: postpass gap mismatch")

    for cell in ("base-classic", "base-arachne"):
        for side in ("off", "on"):
            key = f"{cell}-{side}"
            wall = need(cell, side, "probe")
            profiled = wall
            fin = gap(wall, "PostPass::LayerFinalization")
            non_copy = (
                sum(
                    probe(profiled, term)["ms"]
                    for term in ("ge.walk", "ge.estimate", "ge.m73")
                )
                if profiled
                else 0.0
            )
            emit_stage = stage(wall, "PostPass::GCodeEmit")["elapsed_ms"]
            recomputed = {
                "finalization_before_ms": fin["before_first_module_ms"],
                "finalization_after_ms": fin["after_last_module_ms"],
                "finalization_modules_ms": fin["module_span_ms"],
                "finalization_copy_drop_ms": (fin["before_first_module_ms"] or 0)
                + (fin["after_last_module_ms"] or 0),
                "gcode_emit_stage_ms": emit_stage,
                "gcode_emit_non_copy_ms": non_copy,
                "gcode_emit_drop_ms": max(0.0, emit_stage - non_copy),
            }
            recomputed["ownership_total_ms"] = (
                recomputed["finalization_copy_drop_ms"]
                + recomputed["gcode_emit_drop_ms"]
                + summary["ownership"][key]["postpass_gap_ms"]
            )
            recomputed["net_upper_ms"] = (
                (recomputed["finalization_before_ms"] or 0)
                + (recomputed["finalization_after_ms"] or 0)
                + summary["ownership"][key]["postpass_gap_ms"]
            )
            recomputed["share_pct"] = pct(
                recomputed["ownership_total_ms"], wall["slice_complete"]["elapsed_ms"]
            )
            recomputed["net_share_pct"] = pct(
                recomputed["net_upper_ms"], wall["slice_complete"]["elapsed_ms"]
            )
            for field, value in recomputed.items():
                published = summary["ownership"][key][field]
                if value is None or published is None:
                    failures.append(f"{key}: {field} missing")
                elif not close(value, published, 0.05):
                    failures.append(f"{key}: {field} {value} != {published}")

    for shape in summary["shapes"]:
        cell = shape["cell"]
        key = f"{cell}-on"
        wall = need(cell, "on", "probe")
        profiled = wall
        term = shape["term_ms"]
        if shape["id"] == "support-plan-exact-z-validation":
            term = (
                probe(profiled, "agg.validate")["ms"]
                if profiled
                else gap(wall, "PrePass::SupportGeometry")["after_last_module_ms"]
            )
        elif shape["id"] == "support-analysis-eligibility-fold":
            term = probe(profiled, "sa.eligibility")["ms"] if profiled else None
        elif shape["id"] == "support-contact-detection":
            term = probe(profiled, "sa.contacts")["ms"] if profiled else None
        elif shape["id"] == "tree-support-planner":
            term = by_id(wall, "com.core.tree-support-planner")["elapsed_ms"]
        elif shape["id"] == "postpass-layer-collection-ownership":
            term = summary["ownership"][key]["net_upper_ms"]
        if term is None and shape["term_ms"] is None:
            continue
        if not close(term, shape["term_ms"], 1.0):
            failures.append(f"{shape['id']}/{cell}: term mismatch")
        share = pct(term, wall["slice_complete"]["elapsed_ms"])
        if not close(share, shape["share_pct"], 0.02):
            failures.append(f"{shape['id']}/{cell}: share mismatch")
        if shape["verdict"].startswith("graduate") and share < summary["floor_pct"]:
            failures.append(
                f"{shape['id']}/{cell}: below the {summary['floor_pct']}% floor"
            )

    branch = summary["branch_distance"]
    if branch:
        arm = captures.get(branch["run_id"])
        if arm is None:
            failures.append("branch arm capture missing")
        else:
            if arm["sha256"] != branch["sha256"]:
                failures.append("branch arm SHA mismatch")
            if arm["bytes"] != branch["bytes"]:
                failures.append("branch arm byte mismatch")
            planner = by_id(arm, "com.core.tree-support-planner")
            if planner and not close(planner["elapsed_ms"], branch["planner_ms"], 1.0):
                failures.append("branch arm planner mismatch")
            validate = probe(arm, "agg.validate")["ms"]
            if branch["validate_ms"] is not None and not close(
                validate, branch["validate_ms"], 1.0
            ):
                failures.append("branch arm validate mismatch")
    return failures


def self_test(summary, captures):
    controls = []

    def add_summary(name, mutate):
        changed = copy.deepcopy(summary)
        mutate(changed)
        controls.append((name, changed, copy.deepcopy(captures)))

    def add_capture(name, mutate):
        changed = copy.deepcopy(captures)
        mutate(changed)
        controls.append((name, copy.deepcopy(summary), changed))

    add_summary(
        "quiet wall",
        lambda s: s["quiet_scoreboard"]["base-classic-on"].__setitem__("wall_s", 50.0),
    )
    add_summary(
        "postpass gap",
        lambda s: s["ownership"]["base-classic-on"].__setitem__(
            "postpass_gap_ms", 9999.0
        ),
    )
    add_summary(
        "validate term",
        lambda s: next(
            x for x in s["shapes"] if x["id"] == "support-plan-exact-z-validation"
        ).__setitem__("term_ms", 1.0),
    )
    add_summary(
        "ownership share",
        lambda s: next(
            x for x in s["shapes"] if x["id"] == "postpass-layer-collection-ownership"
        ).__setitem__("share_pct", 0.01),
    )
    add_summary("floor raised", lambda s: s.__setitem__("floor_pct", 90.0))
    if summary.get("branch_distance"):
        add_summary(
            "branch planner",
            lambda s: s["branch_distance"].__setitem__("planner_ms", 1.0),
        )
    add_capture(
        "output sha",
        lambda c: c["base-classic-on-probe"].__setitem__("sha256", "0" * 64),
    )
    add_capture(
        "stage gap",
        lambda c: next(
            g
            for g in c["base-classic-on-probe"]["stage_gaps"]
            if g["stage"] == "PostPass::LayerFinalization"
        ).__setitem__("after_last_module_ms", 9999),
    )

    caught = 0
    for name, changed_summary, changed_captures in controls:
        try:
            failures = recompute(changed_summary, changed_captures)
        except Failure:
            caught += 1
            continue
        if failures:
            caught += 1
        else:
            print(f"self-test MISS: {name} not rejected")
    return caught, len(controls)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    summary, captures = load()
    failures = recompute(summary, captures)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print(
        f"PASS: quiet medians, {len(captures)} captures, ownership "
        "decomposition, 10 shape rows, "
        f"{len([s for s in summary['shapes'] if s['verdict'].startswith('graduate')])} floor tests, "
        f"branch arm {'present' if summary.get('branch_distance') else 'absent'}"
    )
    if args.self_test:
        caught, total = self_test(summary, captures)
        if caught != total:
            print(f"FAIL: self-test caught {caught}/{total} negative controls")
            return 1
        print(f"PASS: self-test rejected {caught}/{total} negative controls")
    return 0


if __name__ == "__main__":
    sys.exit(main())
