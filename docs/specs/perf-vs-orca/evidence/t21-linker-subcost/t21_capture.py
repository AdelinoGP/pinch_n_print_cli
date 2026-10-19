"""Shared parsing helpers for the T21 capture evidence.

Used by `t21_split.py` and `verify-t21.py` so the reducer and the verifier
cannot drift apart on the capture format.
"""

from __future__ import annotations

import json
from pathlib import Path

PROBE_PREFIX = "T21-PROBE"
MODULE_ID = "com.core.infill-linker"


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


def probe_counters(path) -> dict[int, dict[str, int]]:
    """Per-layer `T21-PROBE` counters, from bare lines or `module_log`."""
    counters: dict[int, dict[str, int]] = {}
    with Path(path).open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith(PROBE_PREFIX):
                parts = dict(p.split("=", 1) for p in line.split()[1:])
                counters[int(parts.pop("layer"))] = {
                    k: int(v) for k, v in parts.items()
                }
    for event in iter_events(path):
        if event.get("event") != "module_log":
            continue
        message = event.get("message", "")
        if not message.startswith(PROBE_PREFIX):
            continue
        parts = dict(p.split("=", 1) for p in message.split()[1:])
        if "layer" in parts:
            counters[int(parts.pop("layer"))] = {k: int(v) for k, v in parts.items()}
    return counters


def module_rows(path, module_id: str = MODULE_ID):
    """Per-`module_complete` (layer, elapsed_ms, call_fuel, scopes).

    `scopes` is the raw scope-row list, so callers can read either bucket.
    """
    rows = []
    for event in iter_events(path):
        if (
            event.get("event") != "module_complete"
            or event.get("module_id") != module_id
        ):
            continue
        scopes = event.get("profile_scopes") or {}
        rows.append(
            (
                event.get("layer_index", -1),
                event.get("elapsed_ms"),
                scopes.get("call_fuel", 0),
                scopes.get("scopes", []),
            )
        )
    return rows


def module_summary_row(path, module_id: str = MODULE_ID):
    """The `profile_summary` module row (carries total_fuel / total_wall_ns)."""
    for event in iter_events(path):
        if event.get("event") != "profile_summary":
            continue
        for module in event["profile"]["modules"]:
            if module["module_id"] == module_id:
                return module, event["profile"]
    raise AssertionError(f"no profile_summary row for {module_id} in {path}")
