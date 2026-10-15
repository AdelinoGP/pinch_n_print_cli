#!/usr/bin/env python3
"""Reduce raw t60 host:native attribution captures into the in-repo set.

python reduce.py --raw-root target/t60-host-native   # raw -> captures/ + summary.json
python reduce.py                                     # checkout-only: rebuild summary.json from captures/

The raw tree is optional and gitignored; the reduced set carries everything
``verify.py`` needs. No slices are launched by this script. All figures are
attribution signals (accumulated thread-time, call counts), never wall claims.
"""

import argparse
import collections
import csv
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
BASELINE_BENCHY = (
    REPO / "docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-13/captures"
)
BASELINE_BASE = (
    REPO
    / "docs/specs/perf-vs-orca/evidence/perf-refresh-2026-10-09/reduced/classic-cells.json"
)
QUALIFICATION = (
    REPO / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
)
PROBE_RE = re.compile(r"^\[T60-PROBE\] owner=(\S+) span_ns=(\d+) entries=(\d+)$")
CELL_ORDER = [
    "benchy-classic-off-profile",
    "benchy-classic-on-profile",
    "base-classic-off-profile",
    "base-classic-on-profile",
]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(
        json.dumps(value, indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )


def parse_stderr(path):
    """Split a capture's stderr into JSON events, probe lines and warnings."""
    events = []
    probes = []
    warnings = collections.Counter()
    for line in Path(path).read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("{"):
            events.append(json.loads(line))
            continue
        match = PROBE_RE.match(line)
        if match:
            probes.append(
                {
                    "owner": match.group(1),
                    "span_ns": int(match.group(2)),
                    "entries": int(match.group(3)),
                }
            )
            continue
        if " WARN " in line:
            if "is not closed" in line:
                warnings["open_loop"] += 1
            elif "ERR_MALFORMED_LAYER_MARKER" in line:
                warnings["malformed_layer_marker"] += 1
            else:
                warnings[line.split(" WARN ", 1)[1].split("] ", 1)[-1]] += 1
    return events, probes, warnings


def wall_owners(profile):
    owners = []
    for module in profile["modules"]:
        if module.get("unit") != "wall_ns":
            continue
        owners.append(
            {
                "owner": module["module_id"],
                "calls": module["calls"],
                "total_wall_ns": module["total_wall_ns"],
                "self_wall_ns": module["self_wall_ns"],
                "scopes": [
                    {
                        "scope": scope["scope"],
                        "calls": scope["calls"],
                        "total_wall_ns": scope["total_wall_ns"],
                        "self_wall_ns": scope["self_wall_ns"],
                    }
                    for scope in module["scopes"]
                ],
            }
        )
    return sorted(owners, key=lambda owner: owner["owner"])


def build_capture(raw, row, inputs):
    run_id = row["run_id"]
    err = raw / (run_id + ".stderr.jsonl")
    events, probes, warnings = parse_stderr(err)
    profile = next(e["profile"] for e in events if e.get("event") == "profile_summary")
    phases = {
        e["phase"]: e["elapsed_ms"]
        for e in events
        if e.get("event") == "phase_complete"
    }
    complete = next(e for e in events if e.get("event") == "slice_complete")
    layers = sum(1 for e in events if e.get("event") == "layer_complete")
    metadata = {
        "run_id": run_id,
        "model": row["model"],
        "generator": row["generator"],
        "supports": row["supports"],
        "threads": int(row["threads"]),
        "exit_code": int(row["exit_code"]),
        "wall_seconds": float(row["wall_seconds"]),
        "cpu_seconds": float(row["cpu_seconds"]),
        "cpu_wall_ratio": float(row["cpu_wall_ratio"]),
        "bytes": int(row["bytes"]),
        "sha256": row["sha256"],
        "reference_sha256": row["reference_sha256"],
        "config_sha256": row["config_sha256"],
        "stderr_sha256": row["stderr_sha256"],
        "raw_stderr_sha256": sha(err),
        "status": row["status"],
        "degraded": row["degraded"] == "True",
        "fatal": int(row["fatal"]),
        "non_fatal": int(row["non_fatal"]),
    }
    return {
        "captured_commit": inputs["captured_commit"],
        "exe_sha256": inputs["exe_sha256"],
        "metadata": metadata,
        "layers": layers,
        "elapsed_ms": complete["elapsed_ms"],
        "phases_ms": phases,
        "warnings": dict(warnings),
        "probe_totals": sorted(probes, key=lambda probe: probe["owner"]),
        "owners": wall_owners(profile),
        "fuel_total": profile["fuel_total"],
    }


def baseline_host_native(cell):
    """The pre-take `host:native` scope counts for one cell, from in-repo refreshes."""
    model, generator, supports = cell.split("-")[:3]
    if "benchy" in cell:
        path = BASELINE_BENCHY / f"ordinary-benchy-{generator}-{supports}-profile.json"
        capture = load(path)
        source = "perf-refresh-2026-10-13/captures/" + path.name
        row = next(
            m for m in capture["profile"]["modules"] if m["module_id"] == "host:native"
        )
    else:
        data = load(BASELINE_BASE)
        source = "perf-refresh-2026-10-09/reduced/classic-cells.json"
        row = next(
            r
            for r in data[f"base-{generator}-{supports}"]["profile_host"]
            if r["module_id"] == "host:native"
        )
    return source, {scope["scope"]: scope["calls"] for scope in row["scopes"]}


def reconciliation(capture):
    baseline_source, baseline = baseline_host_native(capture["metadata"]["run_id"])
    by_scope = collections.defaultdict(lambda: {"attributed": 0, "residual": 0})
    for owner in capture["owners"]:
        if owner["owner"].startswith("probe:"):
            bucket = "attributed"
        elif owner["owner"] == "host:native":
            bucket = "residual"
        else:
            # Prepass built-ins have their own brackets; they are not part of
            # the host:native bucket this reconciliation accounts for.
            continue
        for scope in owner["scopes"]:
            by_scope[scope["scope"]][bucket] += scope["calls"]
    rows = []
    for scope in sorted(baseline):
        rows.append(
            {
                "scope": scope,
                "baseline_calls": baseline[scope],
                "attributed_calls": by_scope[scope]["attributed"],
                "residual_calls": by_scope[scope]["residual"],
                "baseline_source": baseline_source,
            }
        )
    return rows


def build_summary(captures):
    cells = {}
    for capture in captures:
        run_id = capture["metadata"]["run_id"]
        table = {}
        for owner in capture["owners"]:
            entry = {
                "total_wall_ms": round(owner["total_wall_ns"] / 1e6, 3),
                "calls": owner["calls"],
            }
            for scope in owner["scopes"]:
                key = scope["scope"].rsplit("::", 1)[-1]
                entry[key + "_calls"] = scope["calls"]
                entry[key + "_wall_ms"] = round(scope["total_wall_ns"] / 1e6, 3)
            table[owner["owner"]] = entry
        cells[run_id] = {
            "output_sha256": capture["metadata"]["sha256"],
            "reference_sha256": capture["metadata"]["reference_sha256"],
            "layers": capture["layers"],
            "phases_ms": capture["phases_ms"],
            "probe_totals": {
                probe["owner"]: {
                    "span_ms": round(probe["span_ns"] / 1e6, 3),
                    "entries": probe["entries"],
                }
                for probe in capture["probe_totals"]
            },
            "owner_table": table,
            "reconciliation": reconciliation(capture),
        }
    return {
        "captured_commit": captures[0]["captured_commit"],
        "exe_sha256": captures[0]["exe_sha256"],
        "cells": cells,
    }


def from_raw(raw):
    inputs = load(raw / "inputs.json")
    rows = list(csv.DictReader((raw / "runs.csv").open(encoding="utf-8-sig")))
    return [build_capture(raw, row, inputs) for row in rows]


def from_repo():
    return [load(HERE / "captures" / f"{run_id}.json") for run_id in CELL_ORDER]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", type=Path, default=None)
    args = parser.parse_args()
    here = args.raw_root
    if here is not None:
        captures = from_raw(here)
        for capture in captures:
            write(
                HERE / "captures" / (capture["metadata"]["run_id"] + ".json"), capture
            )
    else:
        captures = from_repo()
        captures = [
            capture
            for capture in captures
            if capture["metadata"]["status"] != "MISSING"
        ]
    write(HERE / "summary.json", build_summary(captures))
    print("reduced", len(captures), "captures ->", HERE / "summary.json")
    for capture in captures:
        rows = reconciliation(capture)
        for row in rows:
            assert (
                row["baseline_calls"] == row["attributed_calls"] + row["residual_calls"]
            ), row
        print(
            capture["metadata"]["run_id"],
            "| residual host:native:",
            sum(row["residual_calls"] for row in rows),
            "of",
            sum(row["baseline_calls"] for row in rows),
            "calls",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
