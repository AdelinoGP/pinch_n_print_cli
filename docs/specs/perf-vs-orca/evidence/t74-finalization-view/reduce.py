#!/usr/bin/env python3
"""Reduce the ticket-74 probe captures to the committed split set.

The raw captures are gitignored under `target/` (instrumented stderr JSONL from
`run-refresh-base.ps1` with `PNP_T74_PROBE=1`). This script extracts only the
T74 probe lines and the finalization module/phase brackets, so the committed
`split.json` can be verified from a plain checkout.

Usage:
  python reduce.py \
    --baseline-on  <path>/ordinary-base-classic-on-instrumented.stderr.jsonl \
    --baseline-off <path>/ordinary-base-classic-off-instrumented.stderr.jsonl \
    --candidate-on <path>/ordinary-base-classic-on-instrumented.stderr.jsonl \
    --candidate-off <path>/ordinary-base-classic-off-instrumented.stderr.jsonl \
    --output split.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

MODULES = [
    "com.core.overhang-classifier-default",
    "com.core.part-cooling",
    "com.core.skirt-brim",
    "com.core.wipe-tower",
]


def parse_probe(path: Path) -> dict:
    probes = {}
    merges = {}
    drops = {}
    brackets = {}
    phases = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("T74PROBE "):
            fields = dict(
                part.split("=", 1) for part in line[len("T74PROBE ") :].split()
            )
            module = fields.pop("module")
            probes[module] = {
                k: float(v) if "." in v else int(v) for k, v in fields.items()
            }
        elif line.startswith("T74MERGE "):
            fields = dict(
                part.split("=", 1) for part in line[len("T74MERGE ") :].split()
            )
            module = fields.pop("module")
            merges[module] = {
                k: float(v) if "." in v else int(v) for k, v in fields.items()
            }
        elif line.startswith("T74DROP "):
            fields = dict(
                part.split("=", 1) for part in line[len("T74DROP ") :].split()
            )
            module = fields.pop("module")
            drops[module] = {k: float(v) for k, v in fields.items()}
            continue
        elif line.startswith("{"):
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if (
                event.get("event") == "module_complete"
                and event.get("stage") == "PostPass::LayerFinalization"
            ):
                brackets[event["module_id"]] = event["elapsed_ms"]
            elif event.get("event") == "phase_complete":
                phases[event["phase"]] = event["elapsed_ms"]
    modules = []
    for module in MODULES:
        modules.append(
            {
                "module": module,
                "bracket_ms": brackets[module],
                **probes[module],
                **drops[module],
                **merges[module],
            }
        )
    return {
        "module_brackets_ms": brackets,
        "module_sum_ms": sum(brackets.values()),
        "postpass_phase_ms": phases.get("postpass"),
        "modules": modules,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    for arm in ("baseline-on", "baseline-off", "candidate-on", "candidate-off"):
        parser.add_argument(f"--{arm}", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    arms = {}
    qualification = json.loads(
        (
            Path(__file__).parent
            / ".."
            / "t58-finalization-copies"
            / "qualification.json"
        ).read_text(encoding="utf-8")
    )
    references = {entry["cell"]: entry for entry in qualification["cells"]}
    for arm in ("baseline-on", "baseline-off", "candidate-on", "candidate-off"):
        label, supports = arm.rsplit("-", 1)
        raw_path = getattr(args, arm.replace("-", "_"))
        capture = parse_probe(raw_path)
        reference = references[f"ordinary-base-classic-{supports}"]
        arms[arm] = {
            "arm": label,
            "supports": supports,
            "raw_capture": raw_path.name,
            "reference_sha256": reference["sha256"],
            "reference_bytes": reference["bytes"],
            **capture,
        }
    payload = {"schema": 1, "cell": "ordinary-base-classic", "captures": arms}
    args.output.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {args.output} with {len(arms)} captures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
