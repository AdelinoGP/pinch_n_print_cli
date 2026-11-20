#!/usr/bin/env python3
"""t173: reduce the probe stderr streams into the committed census.

Reads `target/t173-probe-runs/<tag>/<cell>.stderr.txt` and writes
`reduced/probe-<tag>.json`. Every number is accumulated thread-time or an
operand count; none is wall. The `--profile` section is parsed only to compare
the probe's clip total with the tool's own clipper scope totals.

Usage:
  python reduce_probe_t173.py [--tag probe]
"""

from __future__ import annotations

import argparse
import collections
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t173-probe-runs"

INT_FIELDS = [
    "calls",
    "empty_calls",
    "empty_thread_ns",
    "subj_points",
    "subj_paths",
    "clip_points",
    "clip_paths",
    "out_points",
    "out_paths",
    "in_points",
    "thread_ns",
    "max_ns",
    "max_in_points",
    "near_points",
    "near_polys",
    "in_bbox_points",
]
STR_FIELDS = ["module", "stage", "site", "entry", "op"]
PCT = re.compile(r"\s(\s*n/a|\s*\d+\.\d+)%\s+(.+?)\s*(?:calls \d+)?$")


def parse_amount(text: str) -> int:
    t = text.strip()
    if t.endswith(" s"):
        return int(round(float(t[:-2]) * 1e9))
    if t.endswith(" ms"):
        return int(round(float(t[:-3]) * 1e6))
    if t.endswith(" us"):
        return int(round(float(t[:-3]) * 1e3))
    if t.endswith(" ns"):
        return int(round(float(t[:-3])))
    return int(t.replace(",", ""))


def parse_profile(text: str) -> dict:
    section = None
    out = {"fuel": {}, "wall": {}}
    for line in text.splitlines():
        if line.startswith("units: fuel"):
            section = "fuel"
            continue
        if line.startswith("units: wall-clock"):
            section = "wall"
            continue
        if line.startswith("=== ") or line.startswith("note:"):
            section = None
            continue
        if section is None or not line.startswith("  "):
            continue
        stripped = line[2:]
        if stripped.startswith("<module self>"):
            continue
        m = PCT.search(line)
        if not m:
            continue
        name = stripped[: m.start(1)].strip()
        if not name or name.startswith("<"):
            continue
        out[section][name] = out[section].get(name, 0) + parse_amount(m.group(2))
    return out


def parse_rows(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        if not line.startswith("t173-row "):
            continue
        fields = {}
        for token in line.split()[1:]:
            if "=" not in token:
                continue
            key, value = token.split("=", 1)
            if key in INT_FIELDS:
                fields[key] = int(value)
            elif key in STR_FIELDS:
                fields[key] = value
        if "calls" in fields:
            rows.append(fields)
    return rows


def parse_batches(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        if not line.startswith("t173-batch "):
            continue
        fields = {}
        for token in line.split()[1:]:
            if "=" not in token:
                continue
            key, value = token.split("=", 1)
            if key in ("batches", "requests", "wall_ns", "max_wall_ns"):
                fields[key] = int(value)
            else:
                fields[key] = value
        rows.append(fields)
    return rows


def reduce_cell(path: Path) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace")
    rows = parse_rows(text)
    batches = parse_batches(text)
    profile = parse_profile(text)
    total_thread = sum(r["thread_ns"] for r in rows)
    cells = {}
    by_site = collections.defaultdict(lambda: collections.Counter())
    by_module = collections.defaultdict(lambda: collections.Counter())
    for r in rows:
        key = (r["module"], r["stage"], r["site"], r["entry"], r["op"])
        for f in INT_FIELDS:
            if f in r:
                by_site[key][f] += r[f]
        mod = (r["module"], r["stage"])
        for f in INT_FIELDS:
            if f in r:
                by_module[mod][f] += r[f]
    site_rows = []
    for key, acc in by_site.items():
        site_rows.append(
            {
                "module": key[0],
                "stage": key[1],
                "site": key[2],
                "entry": key[3],
                "op": key[4],
                **{f: acc[f] for f in INT_FIELDS if f in acc},
            }
        )
    site_rows.sort(key=lambda x: -x["thread_ns"])
    for row in site_rows:
        row["share_of_cell_clip_thread"] = round(row["thread_ns"] / total_thread, 6)
    module_rows = []
    for key, acc in by_module.items():
        module_rows.append(
            {
                "module": key[0],
                "stage": key[1],
                **{f: acc[f] for f in INT_FIELDS if f in acc},
            }
        )
    module_rows.sort(key=lambda x: -x["thread_ns"])
    cells["rows"] = site_rows
    cells["modules"] = module_rows
    cells["batches"] = batches
    cells["total_clip_thread_ns"] = total_thread
    cells["profile_wall_scopes"] = profile
    return cells


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="probe")
    a = ap.parse_args()
    raw_dir = RAW / a.tag
    cells = {}
    for path in sorted(raw_dir.glob("*.stderr.txt")):
        cell = path.name.removesuffix(".stderr.txt")
        cells[cell] = reduce_cell(path)
        print(
            f"{cell}: rows={len(cells[cell]['rows'])} "
            f"clip_thread={cells[cell]['total_clip_thread_ns'] / 1e9:.3f}s",
            flush=True,
        )
    out = {
        "schema": 1,
        "tag": a.tag,
        "note": (
            "Probe census: accumulated thread-time and operand counts from "
            "t173-row lines; never wall. `profile_wall_scopes` is the tool's own "
            "--profile scope section, recorded only for cross-check."
        ),
        "cells": cells,
    }
    out_path = HERE / "reduced" / f"census-{a.tag}.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print("wrote", out_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
