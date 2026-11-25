#!/usr/bin/env python3
"""t131 per-op decomposition -- reduce probe runs into `summary.json`.

Reads `<raw>/<cell>.<flags>.jsonl` (stderr of `pnp_cli slice --instrument-stderr`
with `PNP_T131_PROBE=1`) and writes the committed reduced set.

Two independent streams are joined:

1. The probe's `t131-stage` lines: per prepass stage / phase, the op-level split
   of every `slicer-core::polygon_ops` primitive (calls, WIT marshal, IR->Path64
   conversion, clipper sweep, PolyTree rebuild, operand and result sizes).
2. The runtime's instrumented `stage_complete` / `phase_complete` events: the
   stage and phase walls the probe is attributed against.

Also reduces the `PNP_T131_S5` windowed-offset lines and the `t131-s6op`
singular-offset content census.

Usage:
  python reduce.py [--raw <dir>] [--out summary.json]
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T123 = HERE.parent / "t123-facet-retirement" / "reduced" / "summary.json"
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]

FIELDS = [
    "calls",
    "wit_in_ns",
    "conv_in_ns",
    "sweep_ns",
    "rebuild_ns",
    "wit_out_ns",
    "in_points",
    "in_paths",
    "out_points",
    "out_paths",
]
INT_FIELDS = FIELDS + ["max_in_points"]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_kv(line: str, prefix: str) -> dict | None:
    if not line.startswith(prefix):
        return None
    body = line[len(prefix) :]
    out: dict[str, str] = {}
    for token in body.split():
        if "=" not in token:
            continue
        key, value = token.split("=", 1)
        out[key] = value
    return out


def num(row: dict | None, key: str) -> float:
    if not row or key not in row:
        return 0.0
    try:
        return float(row[key])
    except ValueError:
        return 0.0


def reduce_cell(cell: str, flags: str, raw_dir: Path) -> dict:
    raw = raw_dir / f"{cell}.{flags}.jsonl"
    stages: dict[str, dict[str, dict[str, float]]] = collections.defaultdict(dict)
    stage_walls: dict[str, dict] = collections.defaultdict(
        lambda: {"calls": 0, "sum_ms": 0.0, "max_ms": 0.0, "phase": ""}
    )
    phase_walls: dict[str, float] = {}
    s5_rows: list[dict] = []
    s6_rows: list[dict] = []

    for line in raw.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("{"):
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            ev = event.get("event")
            if ev == "stage_complete":
                stage = event.get("stage", "")
                rec = stage_walls[stage]
                rec["calls"] += 1
                rec["sum_ms"] += event.get("elapsed_ms") or 0
                rec["max_ms"] = max(rec["max_ms"], event.get("elapsed_ms") or 0)
                rec["phase"] = event.get("phase", "")
            elif ev == "phase_complete":
                phase_walls[event.get("phase", "")] = event.get("elapsed_ms") or 0
            continue
        if line.startswith("t131-stage "):
            kv = parse_kv(line, "t131-stage ")
            if not kv:
                continue
            stage = kv["stage"]
            op = kv["op"]
            dest = stages[stage].setdefault(op, {f: 0 for f in INT_FIELDS})
            for field in FIELDS:
                dest[field] += num(kv, field)
            continue
        if line.startswith("t131-s5 "):
            kv = parse_kv(line, "t131-s5 ")
            if kv:
                s5_rows.append(
                    {
                        k: (float(v) if k not in ("region",) else v)
                        for k, v in kv.items()
                    }
                )
            continue
        if line.startswith("t131-s6op "):
            kv = parse_kv(line, "t131-s6op ")
            if kv:
                s6_rows.append(kv)
            continue

    gcode = raw_dir / f"{cell}.{flags}.gcode"
    got = sha256(gcode) if gcode.exists() else ""
    refs = json.loads(T123.read_text(encoding="utf-8"))["output"]
    want = refs[cell]["candidate-ordinary"]["sha256"]

    s5 = None
    if s5_rows:
        neg = [r for r in s5_rows if r.get("neg_ran") == 1.0]
        s5 = {
            "layers": len(s5_rows),
            "neg_ran_layers": len(neg),
            "sum_whole_pos_ms": round(sum(r["whole_pos_ns"] for r in s5_rows) / 1e6, 3),
            "sum_win_pos_ms": round(sum(r["win_pos_ns"] for r in s5_rows) / 1e6, 3),
            "sum_whole_neg_ms": round(sum(r["whole_neg_ns"] for r in neg) / 1e6, 3),
            "sum_win_neg_ms": round(sum(r["win_neg_ns"] for r in neg) / 1e6, 3),
            "sum_points_whole": int(sum(r["pts_whole"] for r in s5_rows)),
            "sum_points_win": int(sum(r["pts_win"] for r in s5_rows)),
            "max_abs_pos_area_delta_mm2": round(
                max(abs(r["area_pos_whole"] - r["area_pos_win"]) for r in s5_rows), 6
            ),
            "max_abs_neg_area_delta_mm2": round(
                max(
                    (abs(r["area_neg_whole"] - r["area_neg_win"]) for r in neg),
                    default=0.0,
                ),
                6,
            ),
            "exact_pos_layers": sum(
                1
                for r in s5_rows
                if abs(r["area_pos_whole"] - r["area_pos_win"]) <= 1e-4
            ),
            "exact_neg_layers": sum(
                1 for r in neg if abs(r["area_neg_whole"] - r["area_neg_win"]) <= 1e-4
            ),
        }

    s6 = None
    if s6_rows:
        by_hash = collections.Counter(r["hash"] for r in s6_rows)
        total_ns = sum(float(r["call_ns"]) for r in s6_rows)
        dup_ns = sum(float(r["call_ns"]) for r in s6_rows if by_hash[r["hash"]] > 1)
        s6 = {
            "calls": len(s6_rows),
            "distinct_payloads": len(by_hash),
            "repeat_calls": len(s6_rows) - len(by_hash),
            "total_ms": round(total_ns / 1e6, 3),
            "repeat_payload_ms": round(dup_ns / 1e6, 3),
            "max_call_ms": round(max(float(r["call_ns"]) for r in s6_rows) / 1e6, 3),
        }

    families: dict[str, dict] = {}
    grouped: dict[tuple[str, str, str], list[dict]] = collections.defaultdict(list)
    for r in s6_rows:
        grouped[(r["delta_mm"], r["miter"], r["arc"])].append(r)
    for (delta, miter, arc), rows in grouped.items():
        hashes = collections.Counter(r["hash"] for r in rows)
        families[f"{delta}|{miter}|{arc}"] = {
            "delta_mm": delta,
            "miter": miter,
            "arc": arc,
            "calls": len(rows),
            "distinct_payloads": len(hashes),
            "repeat_calls": len(rows) - len(hashes),
            "total_ms": round(sum(float(r["call_ns"]) for r in rows) / 1e6, 3),
        }

    return {
        "cell": cell,
        "flags": flags,
        "gcode_sha256": got,
        "reference_sha256": want,
        "reference_match": got == want,
        "phase_walls_ms": phase_walls,
        "stage_walls_ms": {
            k: {
                "calls": v["calls"],
                "sum_ms": v["sum_ms"],
                "max_ms": v["max_ms"],
                "phase": v["phase"],
            }
            for k, v in sorted(stage_walls.items())
        },
        "t131_stage": {
            stage: {op: dict(payload) for op, payload in ops.items()}
            for stage, ops in sorted(stages.items())
        },
        "s5": s5,
        "s6": s6,
        "s6_families": families,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", default=str(REPO / "target" / "t131-runs"))
    ap.add_argument("--out", default=str(HERE / "summary.json"))
    a = ap.parse_args()
    raw_dir = Path(a.raw)

    cells = {}
    for cell in CELLS:
        for flags in ("probe", "probe+s5"):
            if not (raw_dir / f"{cell}.{flags}.jsonl").exists():
                continue
            key = cell if flags == "probe" else f"{cell}@s5"
            cells[key] = reduce_cell(cell, flags, raw_dir)

    payload = {
        "probe": "PNP_T131_PROBE=1 (temporary probe build; reverted)",
        "binary_sha256_note": "probe build, not the tree's reference binary",
        "cells": cells,
    }
    Path(a.out).write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    for key, rec in cells.items():
        print(
            f"{key}: ref_match={rec['reference_match']} "
            f"stages={len(rec['t131_stage'])} s5={bool(rec['s5'])} s6={bool(rec['s6'])}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
