#!/usr/bin/env python3
"""[T62] Reducer for the post-hoist infill-linker attribution captures.

Reconstructs the in-repo reduced set from the raw capture tree (default
``target/t62/raw``) via ``--raw-root``:

- ``profile-modules.json`` — the ``profile_summary`` module table per run,
  restricted to ``com.core.infill-linker`` plus the slice ``fuel_total``.
- ``per-layer.json`` — per-layer T62-PROBE counter lines joined with the
  matching ``module_complete`` ``profile_scopes`` rows (``call_fuel`` plus the
  ``t62::*`` scope rows that matter for the cost model).
- ``output-hashes.json`` — SHA-256 of every captured G-code output.

The verifier (``verify.py``) re-derives the headline attribution tables from
these reduced files alone, without the raw tree.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

LINKER = "com.core.infill-linker"
COUNTER_RE = re.compile(r"T62-PROBE layer=(\d+) fn_us=(\d+) orch_us=(\d+) (.*)$")
COUNTER_KEYS = [
    "regions",
    "matched",
    "roles",
    "selected",
    "locked",
    "invocations",
    "prepare_calls",
    "prep_bnd_pts",
    "clip_calls",
    "clip_in_pts",
    "clip_bnd_pts",
    "clip_out",
    "short_out",
    "source_segments",
    "meta_in_pts",
    "meta_src_pts",
    "fallbacks",
    "empties",
    "endpoints",
    "cands",
    "merges",
    "rings",
    "bpts",
    "unions",
    "regroups",
    "owners",
    "out",
]
SCOPE_KEYS = [
    "t62::execute",
    "t62::engine_add",
    "t62::clip_prepare",
    "t62::connect_infill",
    "t62::meta_convert",
    "t62::clip_loop",
    "t62::clip_call_total",
    "t62::short_meta",
    "t62::subj_build",
    "t62::out_convert",
    "t62::overlap_offset",
    "t62::graph_build",
    "t62::boundary_prep",
    "t62::records_build",
    "t62::bucket_process",
    "t62::group_link",
    "t62::orchestrate_total",
    "t62::output_emit",
    "t62::union_boundary",
    "t62::majority_owner",
    "t62::locked_footprint",
    "polygon_ops::clip_polygons",
]


def parse_stderr(path: Path) -> tuple[dict[int, dict], dict | None]:
    """Returns (per-layer counters, profile_summary profile table)."""
    counters: dict[int, dict] = {}
    profile = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "T62-PROBE" in line and '"module_log"' in line:
            match = COUNTER_RE.search(line)
            if match is None:
                continue
            layer = int(match.group(1))
            entry: dict = {
                "layer": layer,
                "fn_us": int(match.group(2)),
                "orch_us": int(match.group(3)),
            }
            for token in match.group(4).split():
                key, _, value = token.partition("=")
                # The log line is embedded in a JSON message string; strip the
                # trailing `"}` that ends it.
                entry[key] = int(value.rstrip('"}'))
            counters[layer] = entry
        elif '"profile_summary"' in line:
            event = json.loads(line)
            if event.get("event") == "profile_summary":
                profile = event["profile"]
    return counters, profile


def parse_module_events(path: Path, module_id: str) -> dict[int, dict]:
    """Per-layer ``module_complete`` rows for one module with scope extract."""
    layers: dict[int, dict] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if '"module_complete"' not in line:
            continue
        event = json.loads(line)
        if event.get("module_id") != module_id:
            continue
        scopes = event.get("profile_scopes") or {}
        layer = int(event["layer_index"])
        layers[layer] = {
            "call_fuel": scopes.get("call_fuel"),
            "elapsed_ms": event.get("elapsed_ms"),
            "scopes": {
                row["scope"]: {
                    "calls": row["calls"],
                    "self_fuel": row["self_fuel"],
                    "total_fuel": row["total_fuel"],
                }
                for row in scopes.get("scopes", [])
            },
        }
    return layers


def link_profile(profile: dict | None) -> tuple[dict | None, int]:
    if profile is None:
        return None, 0
    modules = profile.get("modules", [])
    module = next((m for m in modules if m["module_id"] == LINKER), None)
    return module, int(profile.get("fuel_total", 0))


def reduce_run(raw_root: Path, run: str) -> dict:
    err = raw_root / f"{run}.stderr.jsonl"
    out = raw_root / f"{run}.gcode"
    counters, profile = parse_stderr(err)
    module_events = parse_module_events(err, LINKER)
    linker, fuel_total = link_profile(profile)
    output_sha = hashlib.sha256(out.read_bytes()).hexdigest() if out.exists() else None

    layers = []
    for layer in sorted(set(counters) | set(module_events)):
        counter = counters.get(layer, {})
        event = module_events.get(layer, {})
        entry = {"layer": layer}
        entry.update(counter)
        entry["call_fuel"] = event.get("call_fuel")
        entry["elapsed_ms"] = event.get("elapsed_ms")
        scopes = event.get("scopes", {})
        for key in SCOPE_KEYS:
            if key in scopes:
                entry[f"fuel_{key}"] = scopes[key]["self_fuel"]
                entry[f"calls_{key}"] = scopes[key]["calls"]
        layers.append(entry)

    return {
        "run": run,
        "fuel_total": fuel_total,
        "linker": {
            "total_fuel": linker.get("total_fuel") if linker else None,
            "calls": linker.get("calls") if linker else None,
            "self_fuel": linker.get("self_fuel") if linker else None,
            "scopes": [
                {
                    "scope": s["scope"],
                    "calls": s["calls"],
                    "self_fuel": s["self_fuel"],
                    "total_fuel": s["total_fuel"],
                }
                for s in linker.get("scopes", [])
            ]
            if linker
            else [],
        },
        "output_sha256": output_sha,
        "layers": layers,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-root", default="target/t62/raw")
    args = parser.parse_args()
    raw_root = Path(args.raw_root)
    if not raw_root.is_absolute():
        # Relative default is anchored at the repo root, not the CWD.
        raw_root = Path(__file__).resolve().parents[5].joinpath(raw_root)
    here = Path(__file__).resolve().parent
    captures = here / "captures"
    captures.mkdir(exist_ok=True)

    runs = [
        "ordinary-benchy-classic-off-profile",
        "ordinary-benchy-classic-on-profile",
        "ordinary-benchy-arachne-off-profile",
        "ordinary-benchy-arachne-on-profile",
        "ordinary-benchy-arachne-off-instrumented",
        "accelerated-benchy-arachne-off-profile",
    ]
    reduced = {run: reduce_run(raw_root, run) for run in runs}
    (captures / "reduced.json").write_text(json.dumps(reduced, indent=1) + "\n")

    hashes = {run: reduced[run]["output_sha256"] for run in runs}
    (captures / "output-hashes.json").write_text(json.dumps(hashes, indent=1) + "\n")

    # Reduced tables the verifier consumes: per-run module split + per-layer rows.
    modules = {}
    for run in runs:
        entry = reduced[run]
        modules[run] = {
            "fuel_total": entry["fuel_total"],
            "linker_total_fuel": entry["linker"]["total_fuel"],
            "linker_scopes": entry["linker"]["scopes"],
            "output_sha256": entry["output_sha256"],
        }
    (captures / "profile-modules.json").write_text(json.dumps(modules, indent=1) + "\n")

    layers = {}
    for run in runs:
        layers[run] = reduced[run]["layers"]
    (captures / "per-layer.json").write_text(json.dumps(layers, indent=1) + "\n")
    print(f"reduced {len(runs)} runs into {captures}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
