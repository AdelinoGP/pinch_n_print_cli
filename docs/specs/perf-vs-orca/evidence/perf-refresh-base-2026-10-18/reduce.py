#!/usr/bin/env python3
"""Reduce optional raw base captures, or re-derive the checkout-only summary.

python reduce.py --raw-root target/perf-refresh-base-2026-10-18
python reduce.py

No slices are launched. Times remain diagnostic; no acceptance verdict is made.
"""

import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
MODEL = "base"
EXPECTED_LAYERS = 495


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def reduce_raw(root):
    write(HERE / "inputs.json", load(root / "inputs.json"))
    for row in csv.DictReader((root / "runs.csv").open(encoding="utf-8-sig")):
        path = root / (row["run_id"] + ".stderr.jsonl")
        events = []
        warnings = collections.Counter()
        for line in path.read_text(encoding="utf-8-sig").splitlines():
            if line.startswith("{"):
                events.append(json.loads(line))
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
        metadata.pop("command")  # Portable command lives in run-refresh-base.ps1.
        for field in ("threads", "exit_code", "bytes", "fatal", "non_fatal"):
            metadata[field] = int(metadata[field])
        for field in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
            metadata[field] = float(metadata[field])
        metadata["degraded"] = metadata["degraded"] == "True"
        metadata["type_counts"] = json.loads(metadata["type_counts"])
        assert metadata["stderr_sha256"] == sha(path)
        assert metadata["sha256"] == sha(root / (row["run_id"] + ".gcode"))
        payload = {
            "metadata": metadata,
            "raw_stderr_sha256": sha(path),
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
            "module_calls": [
                [
                    modules.index(e["module_id"]),
                    e["layer_index"],
                    e["elapsed_ms"],
                    e["timestamp_ms"] - start,
                    e.get("profile_scopes", {}).get("call_fuel"),
                ]
                for e in events
                if e.get("event") == "module_complete" and e.get("phase") == "per_layer"
            ],
            "profile": next(
                (e["profile"] for e in events if e.get("event") == "profile_summary"),
                None,
            ),
            "completion": next(e for e in events if e.get("event") == "slice_complete"),
        }
        write(HERE / "captures" / (row["run_id"] + ".json"), payload)


def summarize(capture):
    meta = capture["metadata"]
    phases = {
        e["phase"]: e["elapsed_ms"]
        for e in capture["phase_events"]
        if e["event"] == "phase_complete"
    }
    stages = {
        e["stage"]: e["elapsed_ms"]
        for e in capture["serial_events"]
        if e["event"] == "stage_complete" and "stage" in e
    }
    calls = collections.defaultdict(list)
    for module, layer, elapsed, end, fuel in capture["module_calls"]:
        calls[capture["module_ids"][module]].append(
            {
                "layer": layer,
                "elapsed_ms": elapsed,
                "end_ms": end,
                "fuel": fuel,
            }
        )
    layers = {v[0]: v for v in capture["layers"]}
    low_end = max(layers[i][2] for i in (1, 2))
    other_end = max(v[2] for i, v in layers.items() if i not in (1, 2))
    layer_elapsed = sorted(
        (
            {"layer": v[0], "elapsed_ms": v[3], "end_ms": v[2]}
            for v in capture["layers"]
        ),
        key=lambda v: v["elapsed_ms"],
        reverse=True,
    )
    per_layer_module_ms = collections.defaultdict(float)
    for module, values in calls.items():
        for c in values:
            per_layer_module_ms[c["layer"]] += c["elapsed_ms"]
    long_tail = sorted(
        (
            {"layer": layer, "module_accumulated_ms": round(ms, 1)}
            for layer, ms in per_layer_module_ms.items()
        ),
        key=lambda v: v["module_accumulated_ms"],
        reverse=True,
    )
    top_calls = sorted(
        (
            {
                "module": module,
                "layer": c["layer"],
                "elapsed_ms": c["elapsed_ms"],
                "end_ms": c["end_ms"],
                "fuel": c["fuel"],
            }
            for module, values in calls.items()
            for c in values
        ),
        key=lambda v: v["elapsed_ms"],
        reverse=True,
    )
    out = {
        "kind": meta["kind"],
        "process_wall_s": meta["wall_seconds"],
        "process_cpu_s": meta["cpu_seconds"],
        "cpu_wall_ratio": meta["cpu_wall_ratio"],
        "slice_elapsed_ms": capture["completion"]["elapsed_ms"],
        "process_minus_slice_s": meta["wall_seconds"]
        - capture["completion"]["elapsed_ms"] / 1000,
        "head_ms": capture["first_event_timestamp_ms"]
        - capture["slice_start_timestamp_ms"],
        "phases_ms": phases,
        "stages_ms": stages,
        "serial_modules_ms": {
            e["module_id"]: e["elapsed_ms"]
            for e in capture["serial_events"]
            if e["event"] == "module_complete"
        },
        "low_layer_intervals_ms": [layers[i] for i in (0, 1, 2)],
        "last_non_low_layer_end_ms": other_end,
        "low_layers_only_finishing_interval_ms": max(0, low_end - other_end),
        "last_layer": max(capture["layers"], key=lambda v: v[2])[0],
        "layer_count": len(capture["layers"]),
        "top_layers_by_elapsed": layer_elapsed[:12],
        "top_layers_by_module_accumulated": long_tail[:12],
        "module_accumulated_ms": {
            name: round(sum(c["elapsed_ms"] for c in values), 1)
            for name, values in calls.items()
        },
        "top_module_calls": top_calls[:25],
        "output": {
            key: meta[key]
            for key in (
                "sha256",
                "bytes",
                "type_counts",
                "status",
                "degraded",
                "fatal",
                "non_fatal",
            )
        },
        "warnings": capture["warnings"],
    }
    profile = capture["profile"]
    if profile:
        total = profile["fuel_total"]
        out["fuel_total"] = total
        out["guest"] = {
            m["module_id"]: {
                "fuel": m["total_fuel"],
                "self_fuel": m["self_fuel"],
                "share_pct": 100 * m["total_fuel"] / total,
                "self_of_module_pct": 100 * m["self_fuel"] / m["total_fuel"],
                "calls": m["calls"],
            }
            for m in profile["modules"]
            if m["unit"] == "fuel"
        }
        out["host_accumulated_ns"] = {
            m["module_id"]: m["total_wall_ns"]
            for m in profile["modules"]
            if m["unit"] == "wall_ns"
        }
        out["host_scopes_ns"] = {
            m["module_id"]: {s["scope"]: s["total_wall_ns"] for s in m["scopes"]}
            for m in profile["modules"]
            if m["unit"] == "wall_ns"
        }
        out["guest_accumulated_ns"] = {
            m["module_id"]: {
                "total": m["total_wall_ns"],
                "self": m["self_wall_ns"],
            }
            for m in profile["modules"]
            if m["unit"] == "fuel"
        }
        linker_fuel = out["guest"].get("com.core.infill-linker", {}).get("fuel")
        if linker_fuel:
            out["linker_low_layer_fuel_share_pct"] = (
                100
                * sum(
                    c["fuel"] or 0
                    for c in calls["com.core.infill-linker"]
                    if c["layer"] in (0, 1, 2)
                )
                / linker_fuel
            )
    return out


def build_summary():
    return {
        path.stem: summarize(load(path))
        for path in sorted((HERE / "captures").glob("*.json"))
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-root", type=Path)
    args = parser.parse_args()
    if args.raw_root:
        reduce_raw(args.raw_root)
    result = build_summary()
    write(HERE / "summary.json", result)
    for name, r in result.items():
        print(
            name,
            "wall",
            round(r["process_wall_s"], 1),
            "phases",
            {k: round(v / 1000, 1) for k, v in r["phases_ms"].items()},
            "last layer",
            r["last_layer"],
            "warnings",
            r["warnings"],
        )
        if "guest" in r:
            leaders = sorted(
                r["guest"].items(), key=lambda kv: kv[1]["share_pct"], reverse=True
            )[:4]
            print(
                "  guest leaders",
                [(k, round(v["share_pct"], 2)) for k, v in leaders],
            )
            hosts = sorted(
                r["host_accumulated_ns"].items(), key=lambda kv: kv[1], reverse=True
            )[:4]
            print("  host leaders", [(k, round(v / 1e9, 2)) for k, v in hosts])


if __name__ == "__main__":
    main()
