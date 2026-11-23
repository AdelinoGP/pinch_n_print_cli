#!/usr/bin/env python3
"""Reduce the t88 raw capture tree into committed per-run capture JSON.

The raw stderr streams live under the gitignored
``target/t88-supports-increment/<batch>/`` tree (see ``run_capture.ps1``).
This script keeps only the terms the evidence needs, so ``verify.py`` can run
from a plain checkout without the raw tree.

    python extract.py --raw target/t88-supports-increment

Batches: one ``t88-wall-<cell>`` (instrumented, probe off) and one
``t88-probe-<cell>`` (instrumented, ``PNP_T88_PROBE=1``) per base cell, plus the
``t88-branch5-*`` config-only arm.
"""

import argparse
import csv
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "captures"

REFERENCE_SHA = {
    "base-classic-off": "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80",
    "base-classic-on": "22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d",
    "base-arachne-off": "01a1434c6ab5bbe965e37985e78137b739b47dccf2d364a5d3dbd0156cad7e97",
    "base-arachne-on": "0b29e1bf928f1c0a160ebdedf08245edfd1d0433b3807ad1118b4578a378981c",
}


def parse_jsonl(path):
    stages = []
    modules = []
    phases = {}
    slice_complete = None
    events = []
    probe = {}
    for line in Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if line.startswith("T88PROBE "):
            fields = dict(
                part.split("=", 1) for part in line.split()[1:] if "=" in part
            )
            term = fields.get("term", "?")
            entry = probe.setdefault(term, {"calls": 0, "ms": 0.0, "loop_ms": 0.0})
            entry["calls"] += 1
            entry["ms"] += float(fields.get("ms", "0"))
            if "loop_ms" in fields:
                entry["loop_ms"] += float(fields["loop_ms"])
            continue
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        events.append(event)
        kind = event.get("event")
        if kind == "stage_complete":
            stages.append({"stage": event["stage"], "elapsed_ms": event["elapsed_ms"]})
        elif kind == "module_complete":
            modules.append(
                {
                    "stage": event["stage"],
                    "module_id": event["module_id"],
                    "elapsed_ms": event["elapsed_ms"],
                }
            )
        elif kind == "phase_complete":
            phases[event["phase"]] = event["elapsed_ms"]
        elif kind == "slice_complete":
            slice_complete = event
    return stages, modules, phases, slice_complete, events, probe


def stage_gaps(events):
    """Per prepass/postpass stage: wall, time before the first module, module
    span, time after the last module. Only single-dispatch stages are useful."""
    gaps = []
    i = 0
    while i < len(events):
        event = events[i]
        if event.get("event") != "stage_start":
            i += 1
            continue
        stage = event["stage"]
        start = event["timestamp_ms"]
        j = i + 1
        module_events = []
        while j < len(events):
            candidate = events[j]
            if candidate.get("event") == "stage_start":
                break
            if (
                candidate.get("event") == "stage_complete"
                and candidate.get("stage") == stage
            ):
                break
            if candidate.get("event") in ("module_start", "module_complete"):
                module_events.append(candidate)
            j += 1
        if not (j < len(events) and events[j].get("event") == "stage_complete"):
            i += 1
            continue
        end = events[j]["timestamp_ms"]
        if not stage.startswith(("PrePass::", "PostPass::")):
            i = j + 1
            continue
        if module_events:
            first = module_events[0]["timestamp_ms"]
            last = module_events[-1]["timestamp_ms"]
            gaps.append(
                {
                    "stage": stage,
                    "wall_ms": end - start,
                    "before_first_module_ms": first - start,
                    "module_span_ms": last - first,
                    "after_last_module_ms": end - last,
                    "modules": sorted(
                        {
                            e["module_id"]
                            for e in module_events
                            if e.get("event") == "module_start"
                        }
                    ),
                }
            )
        else:
            gaps.append(
                {
                    "stage": stage,
                    "wall_ms": end - start,
                    "before_first_module_ms": None,
                    "module_span_ms": None,
                    "after_last_module_ms": None,
                    "modules": [],
                }
            )
        i = j + 1
    return gaps


def aggregate(stages, modules):
    stage_rows = {}
    for row in stages:
        entry = stage_rows.setdefault(row["stage"], {"calls": 0, "elapsed_ms": 0})
        entry["calls"] += 1
        entry["elapsed_ms"] += row["elapsed_ms"]
    module_rows = {}
    for row in modules:
        key = (row["stage"], row["module_id"])
        entry = module_rows.setdefault(key, {"calls": 0, "elapsed_ms": 0})
        entry["calls"] += 1
        entry["elapsed_ms"] += row["elapsed_ms"]
    return (
        [
            {"stage": stage, **values}
            for stage, values in sorted(
                stage_rows.items(), key=lambda kv: -kv[1]["elapsed_ms"]
            )
        ],
        [
            {"stage": stage, "module_id": module_id, **values}
            for (stage, module_id), values in sorted(
                module_rows.items(), key=lambda kv: -kv[1]["elapsed_ms"]
            )
        ],
    )


def read_row(path):
    with open(path, encoding="utf-8-sig", newline="") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise SystemExit(f"{path}: expected exactly one row, got {len(rows)}")
    return rows[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", default="target/t88-supports-increment")
    parser.add_argument("--out", default=str(OUT))
    args = parser.parse_args()
    raw = Path(args.raw)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    written = 0
    for batch in sorted(p for p in raw.iterdir() if p.is_dir()):
        rows_path = batch / "rows.csv"
        if not rows_path.exists():
            continue
        row = read_row(rows_path)
        stderr = batch / "stderr" / f"{row['run_id']}.jsonl"
        if not stderr.exists():
            raise SystemExit(f"missing stderr for {row['run_id']}: {stderr}")
        stages, modules, phases, slice_complete, events, probe = parse_jsonl(stderr)
        stage_rows, module_rows = aggregate(stages, modules)
        inputs = json.loads((batch / "inputs.json").read_text(encoding="utf-8"))
        capture = {
            "batch": batch.name,
            "run_id": row["run_id"],
            "cell": row["cell"],
            "label": row["label"],
            "probe": row.get("probe", "False") == "True",
            "profile": row.get("profile", "False") == "True",
            "captured_commit": inputs["captured_commit"],
            "exe_sha256": row["exe_sha256"],
            "threads": int(row["threads"]),
            "wall_seconds": float(row["wall_seconds"]),
            "cpu_seconds": float(row["cpu_seconds"]),
            "cpu_wall_ratio": float(row["cpu_wall_ratio"]),
            "bytes": int(row["bytes"]),
            "type_counts": json.loads(row["type_counts"]),
            "sha256": row["sha256"],
            "reference_sha256": REFERENCE_SHA.get(
                f"{row['fixture']}-{row['cell'].replace('-supports-', '-')}"
            ),
            "status": row["status"],
            "degraded": row["degraded"].strip().lower() == "true",
            "fatal": int(row["fatal"]),
            "non_fatal": int(row["non_fatal"]),
            "config": row["config"],
            "config_sha256": row["config_sha256"],
            "slice_complete": {
                "elapsed_ms": slice_complete["elapsed_ms"],
                "status": slice_complete["status"],
                "degraded": slice_complete["degraded"],
                "fatal_error_count": slice_complete["fatal_error_count"],
                "non_fatal_error_count": slice_complete["non_fatal_error_count"],
            },
            "phases": phases,
            "stages": stage_rows,
            "modules": module_rows,
            "stage_gaps": [
                gap
                for gap in stage_gaps(events)
                if gap["stage"]
                in {
                    "PrePass::SupportAnalysis",
                    "PrePass::SupportGeometry",
                    "PostPass::LayerFinalization",
                    "PostPass::GCodeEmit",
                    "PostPass::GCodePostProcess",
                    "PostPass::GCodeSerialize",
                }
                and gap["wall_ms"] >= 100
            ],
            "probe_terms": probe,
        }
        path = out / f"{batch.name}__{row['run_id']}.json"
        path.write_text(
            json.dumps(capture, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        written += 1
        print(
            f"wrote {path} (slice {slice_complete['elapsed_ms']} ms, {len(stage_rows)} stages)"
        )
    print(f"{written} capture(s)")


if __name__ == "__main__":
    main()
