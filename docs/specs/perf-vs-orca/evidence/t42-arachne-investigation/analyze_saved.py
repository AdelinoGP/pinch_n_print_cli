"""Read-only replay of T38 retained samples; never launches a slicer or build.

Default exit 0 means evidence validation succeeded, NOT adoption acceptance.
--require-wall-separation exits 1 on the saved Arachne wall overlap.
"""

import argparse
import csv
import hashlib
import json
from collections import Counter
from pathlib import Path
from statistics import mean, median


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stats(values):
    return {
        "min": min(values),
        "max": max(values),
        "mean": mean(values),
        "median": median(values),
        "range": max(values) - min(values),
    }


def analyze(attempt):
    inventory = json.loads((attempt / "hash-inventory.json").read_text())
    summary_path = attempt / "campaign-summary.json"
    require(
        sha256(summary_path)
        == inventory["tracked_evidence_files_excluding_this_inventory"][
            "campaign-summary.json"
        ],
        "runner summary hash mismatch",
    )
    summary = json.loads(summary_path.read_text())
    archive = Path(inventory["durable_raw_archive"])
    hashes = inventory["durable_raw_archive_files_excluding_its_inventory"]
    verified = {}

    def check(relative):
        path = archive / relative
        actual = sha256(path)
        require(actual == hashes[relative], f"archive hash mismatch: {relative}")
        verified[relative] = actual
        return path

    require(
        check("summary.json").read_bytes() == summary_path.read_bytes(),
        "archived summary differs from tracked summary",
    )
    output = {}
    phases = ("validation", "prepass", "per_layer", "postpass")
    for cell in summary["cells"]:
        if cell["decision"] == "not-run":
            continue
        require(
            cell["workload"] == "supports-off-benchy", "unexpected executed workload"
        )
        generator = cell["generator"]
        rows = []
        for variant in ("baseline", "candidate"):
            retained = cell[variant]
            require(
                len(retained["rows"]) == summary["samples_per_cell"], "sample count"
            )
            require(
                [r["cpu_seconds"] for r in retained["rows"]] == retained["cpu_samples"],
                "CPU array differs from retained rows",
            )
            require(
                [r["wall_seconds"] for r in retained["rows"]]
                == retained["wall_samples"],
                "wall array differs from retained rows",
            )
            for saved in retained["rows"]:
                prefix = f"runs/{cell['workload']}-{generator}/{saved['label']}"
                with check(prefix + ".csv").open(
                    encoding="utf-8-sig", newline=""
                ) as stream:
                    csv_rows = list(csv.DictReader(stream))
                require(len(csv_rows) == 1, f"CSV row count: {prefix}")
                raw = csv_rows[0]
                require(
                    raw["label"] == saved["label"]
                    and raw["warmup"] == "0"
                    and int(raw["threads"]) == summary["threads"],
                    "CSV run identity",
                )
                for field in ("cpu_seconds", "wall_seconds"):
                    require(
                        float(raw[field]) == saved[field],
                        f"CSV/summary {field} mismatch",
                    )
                require(
                    raw["completion_status"] == saved["status"] == "ok"
                    and raw["exit_code"] == "0"
                    and raw["expected_generator"]
                    == raw["validated_generator"]
                    == saved["generator_marker"]
                    == generator,
                    "CSV status/generator",
                )
                require(
                    raw["degraded"].lower() == "false"
                    and saved["degraded"] == 0
                    and int(raw["fatal_error_count"]) == saved["fatal_errors"] == 0
                    and int(raw["non_fatal_error_count"])
                    == saved["nonfatal_errors"]
                    == 0,
                    "CSV error counters",
                )
                gcode = check(prefix + ".gcode")
                require(
                    verified[prefix + ".gcode"] == raw["sha256_of_gcode"]
                    and gcode.stat().st_size == int(raw["gcode_bytes"]),
                    "G-code identity",
                )
                with check(prefix + ".jsonl").open(encoding="utf-8") as stream:
                    events = []
                    for line in stream:
                        if line.lstrip().startswith("{"):
                            event = json.loads(line)
                            if "schema_version" in event:
                                events.append(event)
                completions = [e for e in events if e["event"] == "slice_complete"]
                require(len(completions) == 1, "slice completion count")
                complete = completions[0]
                require(
                    complete["status"] == "ok"
                    and not complete["degraded"]
                    and complete["fatal_error_count"]
                    == complete["non_fatal_error_count"]
                    == 0,
                    "trace completion status",
                )
                phase_events = [e for e in events if e["event"] == "phase_complete"]
                require(
                    Counter(e["phase"] for e in phase_events) == Counter(phases),
                    "missing/duplicate phase completion",
                )
                phase_ms = {e["phase"]: e["elapsed_ms"] for e in phase_events}
                layers = [e for e in events if e["event"] == "layer_complete"]
                layer_count = next(
                    e["layer_count"]
                    for e in events
                    if e["event"] == "phase_start" and e["phase"] == "per_layer"
                )
                require(
                    len(layers) == layer_count
                    and {e["layer_index"] for e in layers} == set(range(layer_count)),
                    "missing/duplicate layer completion",
                )
                layer_phase_start = next(
                    e["timestamp_ms"]
                    for e in events
                    if e["event"] == "phase_start" and e["phase"] == "per_layer"
                )
                layer_starts = {
                    e["layer_index"]: e["timestamp_ms"]
                    for e in events
                    if e["event"] == "layer_start"
                }
                require(len(layer_starts) == layer_count, "layer start count")
                longest = sorted(layers, key=lambda e: e["elapsed_ms"], reverse=True)
                slow_indices = {e["layer_index"] for e in longest[:2]}
                layer_details = [
                    {
                        "layer_index": e["layer_index"],
                        "elapsed_ms": e["elapsed_ms"],
                        "start_offset_ms": layer_starts[e["layer_index"]]
                        - layer_phase_start,
                        "complete_offset_ms": e["timestamp_ms"] - layer_phase_start,
                    }
                    for e in longest[:5]
                ]
                wall_ms = saved["wall_seconds"] * 1000
                row = {
                    "label": saved["label"],
                    "variant": variant,
                    "timestamp_utc": raw["timestamp_utc"],
                    "cpu_seconds": saved["cpu_seconds"],
                    "process_wall_ms": wall_ms,
                    "phase_ms": phase_ms,
                    "runtime_elapsed_ms": complete["elapsed_ms"],
                    "runtime_outside_phases_ms": complete["elapsed_ms"]
                    - sum(phase_ms.values()),
                    "process_outside_runtime_ms": wall_ms - complete["elapsed_ms"],
                    "layer_elapsed_sum_ms_not_cpu": sum(
                        e["elapsed_ms"] for e in layers
                    ),
                    "longest_layers": layer_details,
                    "longest_two_elapsed_sum_ms_not_cpu": sum(
                        e["elapsed_ms"] for e in longest[:2]
                    ),
                    "other_layers_elapsed_sum_ms_not_cpu": sum(
                        e["elapsed_ms"] for e in longest[2:]
                    ),
                    "all_except_longest_two_complete_by_ms": max(
                        e["timestamp_ms"] - layer_phase_start
                        for e in layers
                        if e["layer_index"] not in slow_indices
                    ),
                    "gcode_sha256": raw["sha256_of_gcode"],
                    "layer_count": layer_count,
                    "event_counts": dict(Counter(e["event"] for e in events)),
                }
                rows.append(row)
        rows.sort(key=lambda r: r["timestamp_utc"])
        require(
            [r["variant"] for r in rows]
            == [
                "baseline",
                "candidate",
                "candidate",
                "baseline",
                "candidate",
                "baseline",
                "baseline",
                "candidate",
            ],
            "ABBA/BAAB order",
        )
        metrics = {}
        for variant in ("baseline", "candidate"):
            chosen = [r for r in rows if r["variant"] == variant]
            metrics[variant] = {
                key: stats([r[key] for r in chosen])
                for key in (
                    "cpu_seconds",
                    "process_wall_ms",
                    "runtime_elapsed_ms",
                    "runtime_outside_phases_ms",
                    "process_outside_runtime_ms",
                    "layer_elapsed_sum_ms_not_cpu",
                    "longest_two_elapsed_sum_ms_not_cpu",
                    "other_layers_elapsed_sum_ms_not_cpu",
                    "all_except_longest_two_complete_by_ms",
                )
            }
            metrics[variant]["phase_ms"] = {
                phase: stats([r["phase_ms"][phase] for r in chosen]) for phase in phases
            }
        a, b = metrics["baseline"], metrics["candidate"]
        cpu_separated = b["cpu_seconds"]["max"] < a["cpu_seconds"]["min"]
        wall_separated = b["process_wall_ms"]["max"] < a["process_wall_ms"]["min"]
        require(
            cpu_separated == cell["cpu_separated"]
            and wall_separated == cell["wall_separated"],
            "gate differs from summary",
        )
        delta = {
            phase: b["phase_ms"][phase]["mean"] - a["phase_ms"][phase]["mean"]
            for phase in phases
        }
        delta.update(
            {
                key: b[key]["mean"] - a[key]["mean"]
                for key in ("runtime_outside_phases_ms", "process_outside_runtime_ms")
            }
        )
        wall_delta = b["process_wall_ms"]["mean"] - a["process_wall_ms"]["mean"]
        require(abs(sum(delta.values()) - wall_delta) < 1e-6, "mean wall decomposition")
        output[generator] = {
            "rows_in_measured_order": rows,
            "metrics": metrics,
            "candidate_minus_baseline_mean_wall_components_ms": delta,
            "mean_process_wall_delta_ms": wall_delta,
            "cpu_separated": cpu_separated,
            "wall_separated": wall_separated,
            "decision": cell["decision"],
        }
    require(set(output) == {"classic", "arachne"}, "missing control/investigation cell")
    return {
        "kind": "saved-evidence-analysis-not-new-acceptance",
        "campaign_status": summary["status"],
        "summary_sha256": sha256(summary_path),
        "raw_files_verified": verified,
        "cells": output,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--attempt",
        type=Path,
        default=Path(__file__).resolve().parent.parent
        / "t38-adoption-resumed/campaign/attempt-1",
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-wall-separation", action="store_true")
    args = parser.parse_args()
    result = analyze(args.attempt)
    if args.output:
        # Keep original evidence and durable inputs read-only.
        destination = args.output.resolve()
        require(
            not destination.is_relative_to(args.attempt.resolve())
            and not destination.is_relative_to(
                Path(
                    json.loads((args.attempt / "hash-inventory.json").read_text())[
                        "durable_raw_archive"
                    ]
                ).resolve()
            ),
            "output must not overwrite original campaign evidence",
        )
        with destination.open("x", encoding="utf-8") as stream:
            stream.write(json.dumps(result, indent=2) + "\n")
    for generator, cell in result["cells"].items():
        print(
            f"{generator}: CPU separated={cell['cpu_separated']}, wall separated={cell['wall_separated']}"
        )
        print(
            "  mean wall components, candidate - baseline (ms):",
            json.dumps(cell["candidate_minus_baseline_mean_wall_components_ms"]),
        )
    print(
        f"Verified {len(result['raw_files_verified'])} raw files against the original hash inventory."
    )
    if (
        args.require_wall_separation
        and not result["cells"]["arachne"]["wall_separated"]
    ):
        print("FAIL: saved Arachne wall samples do not strictly favor accelerated.")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
