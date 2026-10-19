#!/usr/bin/env python3
"""Ticket-22 probe extraction: read a stderr JSONL capture and print the split.

Probe provenance:
  * guest side — `t22-planner-split` `module_log` line from
    `modules/core-modules/tree-support-planner/src/lib.rs` (`T22Span`),
    microseconds accumulated per named phase.
  * host side — one JSON line `{"probe":"PERF-T22-PROBE",...}` from
    `crates/slicer-wasm-host/src/perf_t22_probe.rs`, emitted once per
    `PrePass::SupportGeometry` stage by `crates/slicer-runtime/src/prepass.rs`.

Both probe families are `PERF-T22-PROBE`-tagged and are removed before commit.

Usage: python t22_split.py <capture.jsonl> [...]
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HOST_PREFIX = '{"probe":"PERF-T22-PROBE"'
SPLIT_RE = re.compile(r"t22-planner-split (.*)")


def parse_guest(blob: str) -> dict[str, dict[str, int]]:
    out: dict[str, dict[str, int]] = {}
    for token in blob.split():
        if "=" not in token:
            continue
        name, value = token.split("=", 1)
        us, _, calls = value.partition("us/")
        try:
            out[name] = {"us": int(us), "calls": int(calls) if calls else 0}
        except ValueError:
            continue
    return out


def extract(path: Path) -> dict:
    guest = None
    host = None
    planner: list[dict] = []
    stage: list[dict] = []
    phases: dict[str, int] = {}
    complete = None
    with path.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if guest is None and "t22-planner-split" in line:
                match = SPLIT_RE.search(line)
                if match:
                    guest = parse_guest(match.group(1))
            if host is None and HOST_PREFIX in line:
                start = line.find(HOST_PREFIX)
                try:
                    host = json.loads(line[start:])
                except json.JSONDecodeError:
                    host = None
            if '"event":"' not in line:
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            kind = event.get("event")
            if (
                kind == "module_complete"
                and event.get("module_id") == "com.core.tree-support-planner"
            ):
                planner.append(
                    {"ms": event.get("elapsed_ms"), "ts": event.get("timestamp_ms")}
                )
            elif (
                kind == "stage_complete"
                and event.get("stage") == "PrePass::SupportGeometry"
            ):
                stage.append(
                    {"ms": event.get("elapsed_ms"), "ts": event.get("timestamp_ms")}
                )
            elif kind == "phase_complete":
                phases[event.get("phase")] = event.get("elapsed_ms")
            elif kind == "slice_complete":
                complete = {
                    "ms": event.get("elapsed_ms"),
                    "degraded": event.get("degraded"),
                    "non_fatal": event.get("non_fatal_error_count"),
                }
    return {
        "file": path.name,
        "dir": path.parent.name,
        "guest_us": guest,
        "host": host,
        "planner_module": planner,
        "stage": stage,
        "phases": phases,
        "slice_complete": complete,
    }


def main() -> None:
    for arg in sys.argv[1:]:
        path = Path(arg)
        if not path.exists():
            print(json.dumps({"file": str(path), "missing": True}))
            continue
        print(json.dumps(extract(path), separators=(",", ":")))


if __name__ == "__main__":
    main()
