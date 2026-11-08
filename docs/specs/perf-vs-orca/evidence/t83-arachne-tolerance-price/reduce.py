#!/usr/bin/env python3
"""Reduce t83 raw captures, or re-derive the checkout-only summary.

    python reduce.py --raw-root target/t83-price/t83-price-1
    python reduce.py

No slices are launched. Instrumented runs are attribution only; the gate
verdict is the ticket's pre-declared pricing gate, not an acceptance verdict.
"""

import argparse
import collections
import csv
import hashlib
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
ARACHNE = "com.core.arachne-perimeters"
WALL_FLOOR_PCT = 0.80  # map below-floor bound, ticket 83 pre-declaration


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def reduce_raw(batch_root):
    write(HERE / "inputs.json", load(batch_root / "inputs.json"))
    for row in csv.DictReader((batch_root / "rows.csv").open(encoding="utf-8-sig")):
        run_id = row["run_id"]
        stderr = (
            batch_root / f"{row['fixture']}-{row['cell']}" / f"{run_id}.stderr.jsonl"
        )
        gcode = batch_root / f"{row['fixture']}-{row['cell']}" / f"{run_id}.gcode"
        events = []
        warnings = collections.Counter()
        for line in stderr.read_text(encoding="utf-8-sig").splitlines():
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
        modules = sorted(
            {
                e["module_id"]
                for e in events
                if e.get("event") == "module_complete" and e.get("phase") == "per_layer"
            }
        )
        metadata = dict(row)
        for field in (
            "threads",
            "exit_code",
            "bytes",
            "fatal",
            "non_fatal",
            "repeat",
            "order_index",
        ):
            metadata[field] = int(metadata[field])
        for field in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
            metadata[field] = float(metadata[field])
        metadata["degraded"] = metadata["degraded"] == "True"
        metadata["type_counts"] = json.loads(metadata["type_counts"])
        assert metadata["stderr_sha256"] == sha(stderr), run_id
        assert metadata["sha256"] == sha(gcode), run_id
        payload = {
            "metadata": metadata,
            "raw_stderr_sha256": sha(stderr),
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
        write(HERE / "captures" / (run_id + ".json"), payload)


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
            {"layer": layer, "elapsed_ms": elapsed, "end_ms": end, "fuel": fuel}
        )
    module_accumulated_ms = {
        name: round(sum(c["elapsed_ms"] for c in values), 1)
        for name, values in calls.items()
    }
    total_per_layer_accum_ms = round(sum(module_accumulated_ms.values()), 1)
    arachne_accum_ms = module_accumulated_ms.get(ARACHNE, 0.0)
    per_layer_ms = phases.get("per_layer", 0.0)
    profile = capture["profile"]
    arachne_fuel = None
    fuel_total = None
    if profile:
        fuel_total = profile["fuel_total"]
        for module in profile["modules"]:
            if module["module_id"] == ARACHNE and module["unit"] == "fuel":
                arachne_fuel = module["total_fuel"]
    return {
        "fixture": meta["fixture"],
        "cell": meta["cell"],
        "arm": meta["arm"],
        "repeat": meta["repeat"],
        "order_index": meta["order_index"],
        "process_wall_s": meta["wall_seconds"],
        "process_cpu_s": meta["cpu_seconds"],
        "cpu_wall_ratio": meta["cpu_wall_ratio"],
        "slice_elapsed_ms": capture["completion"]["elapsed_ms"],
        "phases_ms": phases,
        "per_layer_ms": per_layer_ms,
        "module_accumulated_ms": module_accumulated_ms,
        "arachne_accumulated_ms": arachne_accum_ms,
        "total_per_layer_module_accumulated_ms": total_per_layer_accum_ms,
        "phase_parallelism": (total_per_layer_accum_ms / per_layer_ms)
        if per_layer_ms
        else None,
        "arachne_fuel": arachne_fuel,
        "fuel_total": fuel_total,
        "arachne_fuel_share_pct": (100 * arachne_fuel / fuel_total)
        if (arachne_fuel and fuel_total)
        else None,
        "completion": {
            key: capture["completion"].get(key)
            for key in (
                "status",
                "degraded",
                "fatal_error_count",
                "non_fatal_error_count",
            )
        },
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


def median(values):
    return statistics.median(values) if values else None


def cell_verdict(runs):
    arms = collections.defaultdict(list)
    for run in runs:
        arms[run["arm"]].append(run)
    if set(arms) != {"default", "canonical"}:
        raise SystemExit(f"cell lacks both arms: {sorted(arms)}")
    default, canonical = arms["default"], arms["canonical"]
    out = {}
    for arm, samples in (("default", default), ("canonical", canonical)):
        out[arm] = {
            "runs": len(samples),
            "process_wall_s": median([s["process_wall_s"] for s in samples]),
            "process_cpu_s": median([s["process_cpu_s"] for s in samples]),
            "per_layer_ms": median([s["per_layer_ms"] for s in samples]),
            "arachne_accumulated_ms": median(
                [s["arachne_accumulated_ms"] for s in samples]
            ),
            "total_per_layer_module_accumulated_ms": median(
                [s["total_per_layer_module_accumulated_ms"] for s in samples]
            ),
            "phase_parallelism": median([s["phase_parallelism"] for s in samples]),
            "arachne_fuel": median([s["arachne_fuel"] for s in samples]),
            "arachne_fuel_share_pct": median(
                [s["arachne_fuel_share_pct"] for s in samples]
            ),
            "arachne_fuel_samples": [s["arachne_fuel"] for s in samples],
            "arachne_accum_samples": [s["arachne_accumulated_ms"] for s in samples],
        }
    d_accum = (
        out["default"]["arachne_accumulated_ms"]
        - out["canonical"]["arachne_accumulated_ms"]
    )
    d_fuel = out["default"]["arachne_fuel"] - out["canonical"]["arachne_fuel"]

    # Repeat spread as a percentage of the arm median (fuel is expected to be
    # near-deterministic; elapsed is not).
    def spread_pct(samples):
        med = median(samples)
        return 100 * (max(samples) - min(samples)) / med if med else 0.0

    fuel_noise_pct = max(
        spread_pct(out["default"]["arachne_fuel_samples"]),
        spread_pct(out["canonical"]["arachne_fuel_samples"]),
    )
    module_wall_equivalent_ms = (
        out["default"]["arachne_accumulated_ms"] / out["default"]["phase_parallelism"]
        if out["default"]["phase_parallelism"]
        else None
    )
    out["module_wall_equivalent_ms"] = module_wall_equivalent_ms
    out["module_share_of_wall_pct"] = (
        100 * module_wall_equivalent_ms / (out["default"]["process_wall_s"] * 1000)
        if module_wall_equivalent_ms
        else None
    )
    fuel_pct = (
        100 * d_fuel / out["default"]["arachne_fuel"]
        if out["default"]["arachne_fuel"]
        else None
    )
    out["delta"] = {
        "arachne_accumulated_ms": d_accum,
        "arachne_accumulated_reduction_pct": (
            100 * d_accum / out["default"]["arachne_accumulated_ms"]
            if out["default"]["arachne_accumulated_ms"]
            else None
        ),
        "arachne_fuel": d_fuel,
        "arachne_fuel_reduction_pct": (
            100 * d_fuel / out["default"]["arachne_fuel"]
            if out["default"]["arachne_fuel"]
            else None
        ),
        "fuel_repeat_spread_pct": fuel_noise_pct,
        "fuel_delta_outside_spread": bool(
            d_fuel is not None
            and abs(100 * d_fuel / out["default"]["arachne_fuel"]) > fuel_noise_pct
        ),
        # Measured parallelism converts accumulated worker time into the wall
        # the phase actually spent. The idealized serial figure is the raw
        # accumulated delta, an upper bound that assumes no parallelism.
        "idealized_serial_ceiling_ms": d_accum,
        "idealized_serial_ceiling_pct_of_wall": (
            100 * d_accum / (out["default"]["process_wall_s"] * 1000)
        ),
        "occupancy_normalized_ceiling_ms": (
            d_accum / out["default"]["phase_parallelism"]
            if out["default"]["phase_parallelism"]
            else None
        ),
        "occupancy_normalized_ceiling_pct_of_wall": (
            100
            * (d_accum / out["default"]["phase_parallelism"])
            / (out["default"]["process_wall_s"] * 1000)
            if out["default"]["phase_parallelism"]
            else None
        ),
        # Secondary bound: if every guest instruction cost the same wall, the
        # fuel delta would scale the module's wall-equivalent share directly.
        "fuel_proportional_ceiling_pct_of_wall": (
            abs(fuel_pct) * out["module_share_of_wall_pct"] / 100
            if (fuel_pct is not None and out["module_share_of_wall_pct"] is not None)
            else None
        ),
    }
    out["material"] = bool(
        out["delta"]["occupancy_normalized_ceiling_pct_of_wall"] is not None
        and out["delta"]["occupancy_normalized_ceiling_pct_of_wall"] >= WALL_FLOOR_PCT
        and out["delta"]["fuel_delta_outside_spread"]
    )
    return out


def build_summary():
    runs = [
        summarize(load(path)) for path in sorted((HERE / "captures").glob("*.json"))
    ]
    cells = collections.defaultdict(list)
    for run in runs:
        cells[f"{run['fixture']}-{run['cell']}"].append(run)
    verdicts = {cell: cell_verdict(samples) for cell, samples in sorted(cells.items())}
    return {
        "gate": {
            "wall_floor_pct_of_cell_wall": WALL_FLOOR_PCT,
            "material_in_any_cell": any(v["material"] for v in verdicts.values()),
            "cells": [c for c, v in verdicts.items() if v["material"]],
        },
        "runs": {
            f"{r['fixture']}-{r['cell']}-{r['arm']}-r{r['repeat']}": r for r in runs
        },
        "cells": verdicts,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw-root", type=Path)
    args = parser.parse_args()
    if args.raw_root:
        reduce_raw(args.raw_root)
    result = build_summary()
    write(HERE / "summary.json", result)
    for cell, v in result["cells"].items():
        d = v["delta"]
        print(
            f"{cell}: fuel {v['default']['arachne_fuel'] / 1e9:.3f}B -> "
            f"{v['canonical']['arachne_fuel'] / 1e9:.3f}B ({d['arachne_fuel_reduction_pct']:+.3f}%), "
            f"accum {v['default']['arachne_accumulated_ms'] / 1000:.1f}s -> "
            f"{v['canonical']['arachne_accumulated_ms'] / 1000:.1f}s, "
            f"parallelism {v['default']['phase_parallelism']:.2f}x, "
            f"ceil {d['occupancy_normalized_ceiling_pct_of_wall']:.3f}% wall, "
            f"material={v['material']}"
        )
    print(
        "MATERIAL" if result["gate"]["material_in_any_cell"] else "DROP (below floor)"
    )


if __name__ == "__main__":
    main()
