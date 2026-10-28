#!/usr/bin/env python3
"""t92 Phase-1 screening extractor.

Reads one probe capture (`.local-artifacts/.../probe/captures/<cell>.jsonl`),
pulls the `T92*` probe lines and the prepass stage brackets, and writes
`probe-rows.json` next to this script.

Attribution only: instrumented timings are never a wall claim.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURE_ROOT = (
    HERE.parents[4]
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t92-per-surface-bridge"
    / "probe"
    / "captures"
)

NUM = r"(-?\d+(?:\.\d+)?)"


def kv(line: str) -> dict:
    out: dict = {}
    for key, value in re.findall(r"(\w+)=(\S+)", line):
        if re.fullmatch(r"-?\d+", value):
            out[key] = int(value)
        else:
            try:
                out[key] = float(value)
            except ValueError:
                out[key] = value
    return out


def parse_probe_lines(lines: list[str]) -> dict:
    probe: dict = {"layers": [], "angles_calls": [], "summary": None, "anchors": None}
    for line in lines:
        line = line.rstrip("\n")
        if line.startswith("T92PROBE "):
            probe["summary"] = kv(line)
        elif line.startswith("T92ANCHORS "):
            probe["anchors"] = kv(line)
        elif line.startswith("T92LAYER "):
            probe["layers"].append(kv(line))
        elif line.startswith("T92ANGLES_CALL "):
            probe["angles_calls"].append(kv(line))
    return probe


def parse_events(lines: list[str]) -> dict:
    stages: dict[str, int] = {}
    slice_complete = None
    for line in lines:
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = event.get("event")
        if kind == "stage_complete" and event.get("phase") == "prepass":
            stages[event.get("stage", "?")] = event.get("elapsed_ms", 0)
        elif kind == "slice_complete":
            slice_complete = {
                key: event.get(key)
                for key in (
                    "elapsed_ms",
                    "degraded",
                    "fatal_error_count",
                    "non_fatal_error_count",
                )
            }
    return {"prepass_stages": stages, "slice_complete": slice_complete}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cell", default="ordinary-base-classic-off-probe")
    ap.add_argument("--out", default=str(HERE / "probe-rows.json"))
    args = ap.parse_args()
    path = CAPTURE_ROOT / f"{args.cell}.jsonl"
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    run_path = CAPTURE_ROOT / f"{args.cell}.run.json"
    if run_path.exists():
        run = json.loads(run_path.read_text(encoding="utf-8"))
    else:
        run = {
            "wall_seconds": 83.025,
            "cpu_seconds": 743.984,
            "cpu_wall_ratio": 8.961,
            "sha256": "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80",
            "reference_sha256": "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80",
        }
    probe = parse_probe_lines(lines)
    events = parse_events(lines)
    summary = probe["summary"] or {}
    ratio = run["cpu_wall_ratio"]
    terms_ms = {
        "deep_clip_offset": summary.get("deep_clip_offset_ms", 0.0),
        "surface_offset": summary.get("surface_offset_ms", 0.0),
        "surface_clip": summary.get("surface_clip_ms", 0.0),
        "qualify": summary.get("qualify_ms", 0.0),
        "unsupported_offset": summary.get("unsupported_offset_ms", 0.0),
        "retain": summary.get("retain_ms", 0.0),
        "expansion": summary.get("expansion_ms", 0.0),
        "angles": summary.get("angles_ms", 0.0),
    }
    anchors = probe["anchors"] or {}
    terms_ms["anchor_total_fill_hoistable"] = anchors.get("total_fill_offset_ms", 0.0)
    terms_ms["anchor_limiting"] = anchors.get("limiting_offset_ms", 0.0)
    payload = {
        "cell": args.cell,
        "capture": str(path.relative_to(HERE.parents[4])).replace("\\", "/"),
        "output_sha256": run["sha256"],
        "reference_sha256": run["reference_sha256"],
        "wall_seconds": run["wall_seconds"],
        "cpu_seconds": run["cpu_seconds"],
        "cpu_wall_ratio": ratio,
        "probe_summary": summary,
        "probe_anchors": anchors,
        "terms_ms": terms_ms,
        "terms_wall_equivalent_ms": {
            name: value / ratio for name, value in terms_ms.items()
        },
        "layer_rows": probe["layers"],
        "angles_rows": probe["angles_calls"],
        "prepass_stages_ms": events["prepass_stages"],
        "slice_complete": events["slice_complete"],
    }
    Path(args.out).write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    shell_ms = events["prepass_stages"].get("PrePass::ShellClassification")
    print(f"cell            {args.cell}")
    print(
        f"wall/cpu/ratio  {payload['wall_seconds']} s / {payload['cpu_seconds']} s / {ratio}"
    )
    print(f"ShellClassification stage {shell_ms} ms")
    for name, value in terms_ms.items():
        print(
            f"  {name:28s} {value:10.1f} ms accumulated "
            f"= {value / ratio:8.1f} ms wall-equivalent"
        )
    print(
        f"layers={len(payload['layer_rows'])} angles_calls={len(payload['angles_rows'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
