#!/usr/bin/env python3
"""Extract the `PrePass::SeamPlanning` stage wall from two instrumented runs.

The runs are `pnp_cli slice ... --instrument-stderr` with the baseline and
candidate module trees. The last `stage_complete` event for the stage is the
run's stage wall; `module_complete` is the guest's own bracket.

Usage:
    python stage.py BASELINE.stderr.jsonl CANDIDATE.stderr.jsonl [--json-out OUT.json]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

STAGE = "PrePass::SeamPlanning"


def stage_events(path: Path) -> dict:
    stage_ms = None
    module_ms = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("stage") != STAGE:
            continue
        if event.get("event") == "module_complete":
            module_ms = event.get("elapsed_ms")
        elif event.get("event") == "stage_complete":
            stage_ms = event.get("elapsed_ms")
    if stage_ms is None:
        raise SystemExit(f"no {STAGE} stage_complete event in {path}")
    return {"stage_ms": stage_ms, "module_ms": module_ms, "source": str(path)}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args()
    baseline = stage_events(args.baseline)
    candidate = stage_events(args.candidate)
    saving = baseline["stage_ms"] - candidate["stage_ms"]
    result = {
        "cell": "base-classic-off",
        "baseline": baseline,
        "candidate": candidate,
        "offset_saving_ms": saving,
        "offset_share_of_baseline_stage_pct": round(
            100.0 * saving / baseline["stage_ms"], 2
        ),
    }
    text = json.dumps(result, indent=1)
    print(text)
    if args.json_out:
        args.json_out.write_text(text + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
