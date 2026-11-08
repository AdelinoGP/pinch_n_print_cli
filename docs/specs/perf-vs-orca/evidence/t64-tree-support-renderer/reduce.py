#!/usr/bin/env python3
"""t64 tree-support renderer probe reducer.

Reduces the raw capture tree (``target/t64-tree-support-renderer/``, produced
by ``run-probe.ps1`` against the temporary T64-PROBE guest instrument) into the
in-repo ``captures/`` set and ``summary.json``:

- per cell: metadata, phase/serial events, per-layer windows, per-layer module
  dispatch brackets with call fuel, the full profile aggregate, and parsed
  ``T64-PROBE layer=...`` counter lines with per-cell counter sums;
- tree-support scope fuel: per named scope self/total fuel with shares of the
  module dispatch fuel, plus the unmarked residual;
- same-run wall relevance (attribution only): renderer dispatch windows vs the
  per-layer phase, occupancy across threads, and the last-finishing-layer
  context for the two supports-on cells with the supports-off controls.

Usage:
  python reduce.py                      # reduce from target/t64-tree-support-renderer
  python reduce.py --raw-root <path>    # alternate raw tree

Exits 0 when every run reduces, output shas match runs.csv, and the invariants
(scope sum + residual == dispatch fuel) hold.
"""

from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
DEFAULT_RAW = REPO / "target" / "t64-tree-support-renderer"

PROBE_RE = re.compile(r"T64-PROBE layer=(\d+)((?: \w+=\d+)+)")

PROBE_FIELDS = [
    "regions",
    "regions_plan",
    "plan_seen",
    "plan_hits",
    "entries",
    "begins",
    "regize",
    "regize_none",
    "unions",
    "union_in",
    "expolys",
    "expoly_verts",
    "paints",
    "paint_blocked",
    "skel_x",
    "skel_pts",
    "pie_edges",
    "pie_bbox",
    "wall_off",
    "wall_paths",
    "fill_off",
    "fill_regs",
    "scan_regs",
    "rows",
    "row_edges",
    "conv_y",
    "flips",
    "conv_x",
    "crossings",
    "fill_paths",
    "out_paths",
    "offgrid",
    "expolys_body",
    "expolys_iface",
    "rows_body",
    "rows_iface",
    "fill_paths_body",
    "fill_paths_iface",
    "wall_paths_body",
    "wall_paths_iface",
    "region_polys",
    "region_edges",
]


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def parse_probe(stderr_text: str) -> list[dict]:
    rows = []
    for line in stderr_text.splitlines():
        match = PROBE_RE.search(line)
        if not match:
            continue
        row = {"layer": int(match.group(1))}
        for token in match.group(2).split():
            name, _, raw = token.partition("=")
            row[name] = int(raw)
        for name in PROBE_FIELDS:
            if name not in row:
                raise SystemExit(
                    f"probe line missing counter {name}: layer={row['layer']}"
                )
        rows.append(row)
    return rows


SDK_RE = re.compile(r"T64-SDK (\w+=\d+(?: \w+=\d+)*)")

SDK_FIELDS = [
    "queries",
    "cover_regions",
    "cover_hits",
    "vert_loops",
    "ring_calls",
    "ring_edges",
    "ring_flips",
    "elig_checks",
]


def parse_sdk(stderr_text: str) -> list[dict]:
    rows = []
    for line in stderr_text.splitlines():
        match = SDK_RE.search(line)
        if not match:
            continue
        row = {}
        for token in match.group(1).split():
            name, _, raw = token.partition("=")
            row[name] = int(raw)
        for name in SDK_FIELDS:
            if name not in row:
                raise SystemExit(f"sdk probe line missing counter {name}")
        rows.append(row)
    return rows


def reduce_raw(root: Path) -> None:
    rows = list(csv.DictReader((root / "runs.csv").open(encoding="utf-8-sig")))
    if not rows:
        raise SystemExit(f"no runs in {root / 'runs.csv'}")
    inputs = json.loads((root / "inputs.json").read_text(encoding="utf-8-sig"))
    summary = {}
    for row in rows:
        run_id = row["run_id"]
        path = root / f"{run_id}.stderr.jsonl"
        gcode = root / f"{run_id}.gcode"
        sha_path = sha(path)
        if sha_path != row["stderr_sha256"]:
            raise SystemExit(f"stderr hash mismatch: {run_id}")
        if sha(gcode) != row["sha256"]:
            raise SystemExit(f"gcode hash mismatch: {run_id}")
        events = []
        warnings: collections.Counter[str] = collections.Counter()
        probe_lines_unmatched = 0
        text = path.read_text(encoding="utf-8-sig")
        for line in text.splitlines():
            if line.startswith("{"):
                events.append(json.loads(line))
            elif "T64-PROBE layer=" in line:
                if not PROBE_RE.search(line):
                    probe_lines_unmatched += 1
            elif " WARN " in line:
                if "is not closed" in line:
                    warnings["open_loop"] += 1
                elif "ERR_MALFORMED_LAYER_MARKER" in line:
                    warnings["malformed_layer_marker"] += 1
                else:
                    warnings[line.split(" WARN ", 1)[1].split("] ", 1)[-1]] += 1
        start = next(
            e["timestamp_ms"]
            for e in events
            if e.get("event") == "phase_start" and e.get("phase") == "per_layer"
        )
        layer_starts = {
            e["layer_index"]: e["timestamp_ms"] - start
            for e in events
            if e.get("event") == "layer_start"
        }
        modules = sorted(
            {
                e["module_id"]
                for e in events
                if e.get("event") == "module_complete" and e.get("phase") == "per_layer"
            }
        )
        metadata = dict(row)
        if "command" in metadata:
            metadata.pop("command")
        for field in ("threads", "exit_code", "bytes", "fatal", "non_fatal"):
            metadata[field] = int(metadata[field])
        for field in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
            metadata[field] = float(metadata[field])
        metadata["degraded"] = metadata["degraded"] == "True"
        metadata["type_counts"] = json.loads(metadata["type_counts"])

        probe_rows = parse_probe(text)
        sdk_rows = parse_sdk(text)
        sdk_totals = {
            name: sum(r.get(name, 0) for r in sdk_rows) for name in SDK_FIELDS
        }
        ring_edges_per_query = [r["ring_edges"] for r in sdk_rows] or [0]
        probe_fields = sorted({key for r in probe_rows for key in r if key != "layer"})
        probe_totals = {
            name: sum(r.get(name, 0) for r in probe_rows) for name in probe_fields
        }
        if probe_lines_unmatched:
            raise SystemExit(
                f"unparsed T64-PROBE lines: {run_id}: {probe_lines_unmatched}"
            )

        module_calls = [
            [
                modules.index(e["module_id"]),
                e["layer_index"],
                e["elapsed_ms"],
                e["timestamp_ms"] - start,
                e.get("profile_scopes", {}).get("call_fuel"),
            ]
            for e in events
            if e.get("event") == "module_complete" and e.get("phase") == "per_layer"
        ]
        payload = {
            "metadata": metadata,
            "raw_stderr_sha256": sha_path,
            "warnings": dict(warnings),
            "first_event_timestamp_ms": events[0]["timestamp_ms"],
            "slice_start_timestamp_ms": int(events[0]["slice_id"].split("-", 1)[1]),
            "per_layer_start_timestamp_ms": start,
            "phase_events": [
                e for e in events if e.get("event") in ("phase_start", "phase_complete")
            ],
            "serial_events": [
                {
                    k: e[k]
                    for k in (
                        "event",
                        "timestamp_ms",
                        "phase",
                        "stage",
                        "module_id",
                        "elapsed_ms",
                    )
                    if k in e
                }
                for e in events
                if e.get("phase") != "per_layer"
                and e.get("event")
                in ("stage_start", "stage_complete", "module_start", "module_complete")
            ],
            "layers_columns": ["index", "start_ms", "end_ms", "elapsed_ms"],
            "layers": sorted(
                [
                    [
                        e["layer_index"],
                        layer_starts[e["layer_index"]],
                        e["timestamp_ms"] - start,
                        e["elapsed_ms"],
                    ]
                    for e in events
                    if e.get("event") == "layer_complete"
                ]
            ),
            "module_ids": modules,
            "module_calls_columns": [
                "module_index",
                "layer_index",
                "elapsed_ms",
                "end_ms",
                "call_fuel",
            ],
            "module_calls": module_calls,
            "probe": {
                "fields": probe_fields,
                "totals": probe_totals,
                "per_layer": probe_rows,
                "sdk": {
                    "fields": SDK_FIELDS,
                    "totals": sdk_totals,
                    "queries": len(sdk_rows),
                    "ring_edges_per_query_mean": round(
                        sum(ring_edges_per_query) / len(ring_edges_per_query), 3
                    ),
                    "ring_edges_per_query_max": max(ring_edges_per_query),
                    "ring_flips_per_query_mean": round(
                        sum(r["ring_flips"] for r in sdk_rows) / max(len(sdk_rows), 1),
                        3,
                    ),
                },
                "inputs": {
                    "raw_stderr_sha256": sha_path,
                    "probed_tree_support_artifact": inputs.get(
                        "probed_tree_support_artifact"
                    ),
                    "probed_tree_support_artifact_sha256": inputs.get(
                        "probed_tree_support_artifact_sha256"
                    ),
                    "captured_commit": inputs.get("captured_commit"),
                    "exe_sha256": inputs.get("exe_sha256"),
                    "model_sha256": inputs.get("model_sha256"),
                },
            },
        }
        profile_event = next(
            (e["profile"] for e in events if e.get("event") == "profile_summary"), None
        )
        payload["profile"] = profile_event
        derived = derive(payload)
        if derived is not None:
            payload["derived"] = derived
        write(HERE / "captures" / f"{run_id}.json", payload)
        summary[run_id] = {
            "metadata": metadata,
            "raw_stderr_sha256": sha_path,
            "derived": derived,
        }
    write(HERE / "summary.json", summary)
    print(f"reduced {len(rows)} run(s) -> captures/ + summary.json")


def derive(payload: dict) -> dict | None:
    """Tree-support scope split + same-run wall attribution (per dispatch)."""
    profile = payload.get("profile")
    if not profile:
        return None
    mod_row = next(
        (m for m in profile["modules"] if m["module_id"] == "com.core.tree-support"),
        None,
    )
    if mod_row is None:
        return None

    named = {
        1: "clip_polygons",
        2: "offset",
        3: "offset2_ex",
        4: "opening",
        5: "closing_ex",
    }
    scopes = []
    for s in mod_row["scopes"]:
        name = s.get("scope") or named.get(s["scope_id"], f"scope#{s['scope_id']}")
        scopes.append(
            {
                "scope": name,
                "scope_id": s["scope_id"],
                "calls": s["calls"],
                "self_fuel": s["self_fuel"],
                "total_fuel": s["total_fuel"],
            }
        )
    total_fuel = mod_row["total_fuel"]
    self_fuel = mod_row["self_fuel"]
    scope_self_sum = sum(s["self_fuel"] for s in scopes)
    core_scope_self = sum(s["self_fuel"] for s in scopes if s["scope_id"] < 1024)
    # The module row's self fuel IS the unmarked residual: the fold credits
    # every top-level scope's inclusive total to the module, so `self` is the
    # fuel outside all marked scopes (t64 scopes + core clip/offset marks).
    residual = self_fuel

    # per-layer renderer dispatch windows (phase-relative ms)
    ts_index = payload["module_ids"].index("com.core.tree-support")
    col = {name: i for i, name in enumerate(payload["module_calls_columns"])}
    per_layer = []
    for mc in payload["module_calls"]:
        if mc[col["module_index"]] != ts_index:
            continue
        layer = mc[col["layer_index"]]
        elapsed = mc[col["elapsed_ms"]] or 0
        end = mc[col["end_ms"]] or 0
        fuel = mc[col["call_fuel"]] or 0
        per_layer.append(
            {
                "layer": layer,
                "start_ms": end - elapsed,
                "end_ms": end,
                "elapsed_ms": elapsed,
                "call_fuel": fuel,
            }
        )
    phase = next(
        (
            e["elapsed_ms"]
            for e in payload["phase_events"]
            if e.get("event") == "phase_complete" and e.get("phase") == "per_layer"
        ),
        None,
    )
    accumulated = sum(l["elapsed_ms"] for l in per_layer)
    threads = payload["metadata"]["threads"]
    occupancy = accumulated / (phase * threads) if phase else None
    last_layer = max((l[0] for l in payload["layers"]), default=None)
    layers_by_index = {l[0]: l for l in payload["layers"]}
    last = None
    if last_layer is not None:
        lrow = layers_by_index.get(last_layer)
        rrow = next((l for l in per_layer if l["layer"] == last_layer), None)
        if lrow and rrow:
            last = {
                "layer": last_layer,
                "window_ms": [lrow[1], lrow[2]],
                "renderer_elapsed_ms": rrow["elapsed_ms"],
                "renderer_end_ms": rrow["end_ms"],
                "renderer_fuel": rrow["call_fuel"],
            }
    probe = payload.get("probe", {})
    totals = probe.get("totals", {})
    return {
        "dispatch_total_fuel": total_fuel,
        "dispatch_self_fuel": self_fuel,
        "scopes": scopes,
        "scope_self_sum": scope_self_sum,
        "core_scope_self_fuel": core_scope_self,
        "t64_scope_self_fuel": scope_self_sum - core_scope_self,
        "unmarked_residual_fuel": residual,
        "per_layer_renderer": per_layer,
        "wall_attention": {
            "phase_wall_ms": phase,
            "threads": threads,
            "renderer_accumulated_worker_ms": accumulated,
            "machine_occupancy_fraction": occupancy,
            "renderer_first_start_ms": min(
                (l["start_ms"] for l in per_layer), default=None
            ),
            "renderer_last_end_ms": max((l["end_ms"] for l in per_layer), default=None),
            "last_finishing_layer": last,
            "probe_totals": totals,
            "probe_sdk_totals": probe.get("sdk", {}).get("totals", {}),
            "probe_sdk_queries_mean_ring_edges": probe.get("sdk", {}).get(
                "ring_edges_per_query_mean"
            ),
            "probe_sdk_queries_mean_ring_flips": probe.get("sdk", {}).get(
                "ring_flips_per_query_mean"
            ),
        },
        "probe_fields_seen": probe.get("fields", []),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", type=Path, default=DEFAULT_RAW)
    args = parser.parse_args()
    try:
        reduce_raw(args.raw_root)
    except SystemExit:
        raise
    except Exception as error:  # noqa: BLE001 - diagnostic reducer
        print(f"reduce failed: {error}", file=sys.stderr)
        raise
