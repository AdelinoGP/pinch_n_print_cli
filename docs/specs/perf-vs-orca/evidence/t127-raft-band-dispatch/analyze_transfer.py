#!/usr/bin/env python3
"""Does the declined raft-fill dispatch cost move to the next Infill module?

Ticket 126 measured `com.core.raft-default` at 61-63 ms per layer on the base
cells while its body returned before doing any work, and priced the saving as
that dispatch's cost. This reducer tests the transfer question directly from
same-session instrumented streams of both arms:

- the per-layer phase wall of each arm;
- which `Layer::Infill` module is dispatched *first* on each layer, and that
  module's elapsed distribution;
- the stage's total thread time.

If the decline merely moves the stage's shared preparation to the next module,
the first-module cost is unchanged and the phase wall does not fall.
"""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
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


def events(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def summarize(raw: Path) -> dict:
    stream = events(raw)
    phase_open = {}
    phase_ms = {}
    for event in stream:
        if event.get("event") == "phase_start":
            phase_open[event["phase"]] = event["timestamp_ms"]
        elif event.get("event") == "phase_complete":
            started = phase_open.get(event["phase"])
            if started is not None:
                phase_ms[event["phase"]] = event["timestamp_ms"] - started

    infill_start_order = defaultdict(list)
    completions = []
    for event in stream:
        if event.get("phase") != "per_layer":
            continue
        if (
            event.get("event") == "module_start"
            and event.get("stage") == "Layer::Infill"
        ):
            infill_start_order[event["layer_index"]].append(
                (event["timestamp_ms"], event["module_id"])
            )
        elif event.get("event") == "module_complete":
            completions.append(event)

    # A stage's shared preparation is paid by whichever Infill module is
    # dispatched first on that layer. Ties stay together: every tied module is
    # a first dispatch on that layer.
    first_on_layer = set()
    for layer, entries in infill_start_order.items():
        earliest = min(ts for ts, _ in entries)
        for ts, module in entries:
            if ts == earliest:
                first_on_layer.add((layer, module))

    first_module = defaultdict(list)
    infill_thread_ms = 0.0
    infill_calls = 0
    for event in completions:
        if event.get("stage") != "Layer::Infill":
            continue
        module = event["module_id"]
        elapsed = float(event.get("elapsed_ms", 0))
        infill_thread_ms += elapsed
        infill_calls += 1
        if (event["layer_index"], module) in first_on_layer:
            first_module[module].append(elapsed)

    first = {}
    for module, values in first_module.items():
        ordered = sorted(values)
        first[module] = {
            "layers": len(values),
            "p50_ms": ordered[len(ordered) // 2],
            "mean_ms": round(statistics.mean(values), 2),
        }
    return {
        "per_layer_phase_ms": phase_ms.get("per_layer", 0),
        "infill_thread_ms": round(infill_thread_ms, 1),
        "infill_calls": infill_calls,
        "first_infill_modules": first,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline-dir", default="target/t127/instrumented-baseline")
    ap.add_argument("--candidate-dir", default="target/t127/instrumented")
    ap.add_argument("--out", default=str(HERE / "reduced" / "transfer.json"))
    args = ap.parse_args()

    baseline_dir = Path(args.baseline_dir)
    candidate_dir = Path(args.candidate_dir)
    cells = {}
    for cell in CELLS:
        base = summarize(baseline_dir / f"{cell}.jsonl")
        cand = summarize(candidate_dir / f"{cell}.jsonl")
        cells[cell] = {
            "baseline": base,
            "candidate": cand,
            "per_layer_phase_delta_ms": cand["per_layer_phase_ms"]
            - base["per_layer_phase_ms"],
            "infill_thread_delta_ms": round(
                cand["infill_thread_ms"] - base["infill_thread_ms"], 1
            ),
        }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(
        json.dumps(
            {
                "ticket": "127",
                "note": (
                    "Same-session instrumented passes of both arms, one per cell. "
                    "`first_infill_modules` lists, per module, the layers on which it "
                    "was the first Layer::Infill dispatch and the elapsed_ms of those "
                    "calls. Attribution only: no wall claim is taken from an "
                    "instrumented pass."
                ),
                "cells": cells,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {out_path}")
    for cell, row in cells.items():
        base_first = ", ".join(
            f"{module} p50 {stats['p50_ms']}ms"
            for module, stats in sorted(row["baseline"]["first_infill_modules"].items())
        )
        cand_first = ", ".join(
            f"{module} p50 {stats['p50_ms']}ms"
            for module, stats in sorted(
                row["candidate"]["first_infill_modules"].items()
            )
        )
        print(f"  {cell}")
        print(f"    baseline first: {base_first}")
        print(f"    candidate first: {cand_first}")
        print(
            f"    per-layer phase {row['baseline']['per_layer_phase_ms']} -> "
            f"{row['candidate']['per_layer_phase_ms']} ms"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
