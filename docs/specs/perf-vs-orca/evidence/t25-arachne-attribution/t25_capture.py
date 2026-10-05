"""Shared parsing helpers for the T25 capture evidence.

Used by `t25_split.py` and `verify-t25.py` so the reducer and the verifier
cannot drift apart on the probe format.
"""

from __future__ import annotations

import json
from pathlib import Path

PROBE_PREFIX = "T25-PROBE"
MODULE_ID = "com.core.arachne-perimeters"


def parse_probe_lines(path) -> list[dict]:
    """`T25-PROBE` rows: [{key: (wall_ns, calls), "seq": int}]."""
    rows = []
    with Path(path).open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            line = line.strip()
            if not line.startswith(PROBE_PREFIX):
                continue
            fields = {}
            seq = None
            for part in line.split()[1:]:
                key, value = part.split("=", 1)
                if key == "seq":
                    seq = int(value)
                else:
                    wall, calls = value.split(":")
                    fields[key] = (int(wall), int(calls))
            rows.append({"seq": seq, **fields})
    return rows


def iter_events(path):
    with Path(path).open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                continue


def module_events(path, event: str = "module_complete", module_id: str = MODULE_ID):
    return [
        e
        for e in iter_events(path)
        if e.get("event") == event and e.get("module_id") == module_id
    ]


def module_scopes(path, module_id: str = MODULE_ID):
    """Latest `profile_summary` row for the module: (module row, {scope: row})."""
    for event in iter_events(path):
        if event.get("event") != "profile_summary":
            continue
        for module in event["profile"]["modules"]:
            if module["module_id"] == module_id:
                return module, {s["scope"]: s for s in module.get("scopes", [])}
    raise RuntimeError(f"no profile_summary row for {module_id} in {path}")
