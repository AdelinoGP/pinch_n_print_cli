#!/usr/bin/env python3
"""t104 phase-split take -- reduce raw instrumented event streams.

Reads `<raw>/<cell>.jsonl` (stderr of `pnp_cli slice --instrument-stderr`) and
writes the committed reduced set `phase-split.json`: per cell, the phase walls,
the prepass stage walls (prepass stages are serial brackets, so their sum
reconciles with the phase), the per-layer thread-summed stage totals
(informational only -- those run in parallel and never denominate wall), the
postpass stages, the outside-every-phase residual, layer count and the
instrumented run's output SHA-256.

Usage:
  python reduce.py [--raw <dir>] [--out phase-split.json]
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]


def load_events(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return rows


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def reduce_cell(raw: Path, gcode: Path) -> dict:
    rows = load_events(raw)
    phases = {
        r["phase"]: r.get("elapsed_ms")
        for r in rows
        if r.get("event") == "phase_complete"
    }
    complete = next(r for r in rows if r.get("event") == "slice_complete")
    stats = next(r for r in rows if r.get("event") == "slice_stats")

    stage_ms: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for r in rows:
        if r.get("event") != "stage_complete" or not r.get("stage"):
            continue
        phase = r["phase"]
        stage_ms[phase][r["stage"]] = stage_ms[phase].get(r["stage"], 0) + (
            r.get("elapsed_ms") or 0
        )

    inner = sum(v for v in phases.values() if v)
    return {
        "raw_events": len(rows),
        "slice_complete_ms": complete["elapsed_ms"],
        "phases_ms": phases,
        "outside_phases_ms": complete["elapsed_ms"] - inner,
        "layer_count": stats["layer_count"],
        "degraded": complete.get("degraded"),
        "fatal_error_count": complete.get("fatal_error_count"),
        "non_fatal_error_count": complete.get("non_fatal_error_count"),
        "prepass_stages_ms": dict(
            sorted(stage_ms["prepass"].items(), key=lambda kv: -kv[1])
        ),
        "per_layer_stages_thread_sum_ms": dict(
            sorted(stage_ms["per_layer"].items(), key=lambda kv: -kv[1])
        ),
        "postpass_stages_ms": dict(
            sorted(stage_ms["postpass"].items(), key=lambda kv: -kv[1])
        ),
        "gcode_sha256": sha256(gcode),
        "gcode_bytes": gcode.stat().st_size,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", default=str(REPO / "target" / "t104-instrumented"))
    ap.add_argument("--gcode-dir", default=str(REPO / "target" / "t104-instrumented"))
    ap.add_argument("--out", default=str(HERE / "phase-split.json"))
    a = ap.parse_args()

    raw_dir = Path(a.raw)
    runs: dict[str, dict] = {}
    for raw in sorted(raw_dir.glob("*.jsonl")):
        gcode = Path(a.gcode_dir) / (raw.stem + ".gcode")
        if not gcode.exists():
            raise SystemExit(f"missing gcode for {raw.stem}: {gcode}")
        runs[raw.stem] = reduce_cell(raw, gcode)

    # A `<cell>.rep2` stream is a repeat of `<cell>` (share-stability check under
    # load). The primary stream is the cell's own name; repeats are folded in.
    cells: dict[str, dict] = {}
    for stem, red in runs.items():
        if stem.endswith(".rep2"):
            continue
        reps = [
            (k.split(".")[-1], v) for k, v in runs.items() if k.startswith(stem + ".")
        ]
        if reps:
            elapsed = red["slice_complete_ms"]
            base_shares = {p: (v or 0) / elapsed for p, v in red["phases_ms"].items()}
            deltas = []
            for _, rep in reps:
                rep_shares = {
                    p: (v or 0) / rep["slice_complete_ms"]
                    for p, v in rep["phases_ms"].items()
                }
                deltas.append(
                    max(abs(base_shares[p] - rep_shares[p]) for p in base_shares)
                )
            red["repeat_count"] = len(reps)
            red["repeat_max_share_delta_pp"] = round(100.0 * max(deltas), 2)
        cells[stem] = red

    out = {
        "note": "Reduced by reduce.py from gitignored raw event streams in target/t104-instrumented/.",
        "cells": cells,
    }
    Path(a.out).write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}: {len(cells)} cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
