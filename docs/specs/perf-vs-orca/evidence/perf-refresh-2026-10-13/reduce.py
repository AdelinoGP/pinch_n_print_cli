#!/usr/bin/env python3
"""Reduce optional raw Benchy captures, or re-derive the checkout-only summary.

python reduce.py --raw-root target/perf-refresh-2026-10-13
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
        metadata.pop("command")  # Portable command lives in run-refresh.ps1.
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
        "serial_modules_ms": {
            e["module_id"]: e["elapsed_ms"]
            for e in capture["serial_events"]
            if e["event"] == "module_complete"
        },
        "low_layer_intervals_ms": [layers[i] for i in (0, 1, 2)],
        "last_non_low_layer_end_ms": other_end,
        "low_layers_only_finishing_interval_ms": max(0, low_end - other_end),
        "last_layer": max(capture["layers"], key=lambda v: v[2])[0],
        "module_accumulated_ms": {
            name: sum(c["elapsed_ms"] for c in values) for name, values in calls.items()
        },
        "linker_low_layer_calls": sorted(
            [c for c in calls["com.core.infill-linker"] if c["layer"] in (0, 1, 2)],
            key=lambda c: c["layer"],
        ),
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
            }
            for m in profile["modules"]
            if m["unit"] == "fuel"
        }
        linker_fuel = out["guest"]["com.core.infill-linker"]["fuel"]
        out["linker_low_layer_fuel_share_pct"] = (
            100 * sum(c["fuel"] for c in out["linker_low_layer_calls"]) / linker_fuel
        )
        out["host_accumulated_ns"] = {
            m["module_id"]: m["total_wall_ns"]
            for m in profile["modules"]
            if m["unit"] == "wall_ns"
        }
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
            "phases",
            r["phases_ms"],
            "last layer",
            r["last_layer"],
            "low-only finish ms",
            r["low_layers_only_finishing_interval_ms"],
            "warnings",
            r["warnings"],
        )
        if "guest" in r:
            print(
                "  guest leaders",
                [(k, round(v["share_pct"], 2)) for k, v in r["guest"].items()][:4],
                "linker L0-2 %",
                round(r["linker_low_layer_fuel_share_pct"], 2),
            )


if __name__ == "__main__":
    main()
