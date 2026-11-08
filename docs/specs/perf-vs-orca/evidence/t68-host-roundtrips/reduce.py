#!/usr/bin/env python3
"""t68 raw-tree reducer (wayfinder perf-vs-orca ticket 68).

Reads the raw capture tree (``target/t68-host-roundtrips/``) and emits the
in-repo reduced set (``captures/*.json`` + ``summary.json``):

- per-cell metadata from ``runs.csv`` (wall/cpu/hashes/status) and
  ``inputs.json`` (commit, exe/guest hashes, provenance);
- phase walls from the ``--instrument-stderr`` JSONL phase events;
- the T68-PROBE host dump (``[T68-PROBE] host owner=... seq=...``) parsed
  into per-site rows: service, sample arg, calls, input/output polygon and
  vertex totals, and the conversion/ops/conversion wall split (ns);
- the guest per-layer ``module_log`` dumps (``[T68-PROBE] layer=...``) summed
  per site (guest-observed full round-trip, us) plus the final layer's row;
- the rendered ``host:native`` table block from the same stderr as the
  run-level bucket cross-check.

Usage: ``python docs/specs/perf-vs-orca/evidence/t68-host-roundtrips/reduce.py [raw_root]``
"""

import csv
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

HOST_RE = re.compile(
    r"\[T68-PROBE\] host owner=(?P<owner>\S+) seq=(?P<seq>\d+) service=(?P<service>\S+) "
    r"arg=(?P<arg>[-\d.]+) calls=(?P<calls>\d+) in_polys=(?P<in_polys>\d+) "
    r"in_verts=(?P<in_verts>\d+) out_polys=(?P<out_polys>\d+) out_verts=(?P<out_verts>\d+) "
    r"conv_in_ns=(?P<conv_in>\d+) ops_ns=(?P<ops>\d+) conv_out_ns=(?P<conv_out>\d+)"
)
GUEST_RE = re.compile(r"ns=\[([^\]]*)\] calls=\[([^\]]*)\]")
TABLE_ROW_RE = re.compile(
    r"^(?P<name>\S+)\s+wall\s+[\d.]+%\s+(?P<val>[\d.]+)\s+(?P<unit>us|ms|s)\s+calls\s+(?P<calls>[\d,]+)\s*$"
)
TABLE_SCOPE_RE = re.compile(
    r"^\s+(?P<name>\S+)\s+[\d.]+%\s+(?P<val>[\d.]+)\s+(?P<unit>us|ms|s)\s+calls\s+(?P<calls>[\d,]+)\s*$"
)

SITE_NAMES = {
    0: "A0 outer-loop inset",
    1: "A1 inner-loop inset",
    2: "B gap shrunk_prev",
    3: "C gap grown_cur",
    4: "D gap difference",
    5: "E infill_area",
    6: "F shrunk_inner",
    7: "G grown_infill",
    8: "H final-transition difference",
    9: "J gap width-band filter",
    10: "K infill-boundary inset",
    11: "thin-wall protrusion clip",
}


def to_ns(value: float, unit: str) -> int:
    return int(round(value * {"us": 1_000, "ms": 1_000_000, "s": 1_000_000_000}[unit]))


def parse_host_table(text: str, name: str):
    """Parse the rendered profile block for `name` (e.g. ``host:native``)."""
    rows = []
    header = None
    for line in text.splitlines():
        m = TABLE_ROW_RE.match(line)
        if m and m.group("name") == name:
            header = {
                "calls": int(m.group("calls").replace(",", "")),
                "total_wall_ns": to_ns(float(m.group("val")), m.group("unit")),
                "scopes": [],
            }
            continue
        if header is not None:
            m = TABLE_SCOPE_RE.match(line)
            if m and not m.group("name").startswith("host"):
                header["scopes"].append(
                    {
                        "scope": m.group("name"),
                        "calls": int(m.group("calls").replace(",", "")),
                        "total_wall_ns": to_ns(float(m.group("val")), m.group("unit")),
                    }
                )
            elif line.strip() == "" or not line.startswith(" "):
                break
    return header


def reduce_cell(raw: Path, run_row: dict, inputs: dict) -> dict:
    run_id = run_row["run_id"]
    stderr_path = raw / f"{run_id}.stderr.jsonl"
    text = stderr_path.read_text(encoding="utf-8", errors="replace")

    host_sites = []
    guest_ns = [0] * 12
    guest_calls = [0] * 12
    dumps = 0
    phases = {}
    last_layer_row = None
    for line in text.splitlines():
        if line.startswith("[T68-PROBE] host"):
            m = HOST_RE.match(line)
            assert m, line
            host_sites.append(
                {
                    "owner": m.group("owner"),
                    "seq": int(m.group("seq")),
                    "site": SITE_NAMES.get(int(m.group("seq")), "?"),
                    "service": m.group("service"),
                    "sample_arg": float(m.group("arg")),
                    "calls": int(m.group("calls")),
                    "in_polys": int(m.group("in_polys")),
                    "in_verts": int(m.group("in_verts")),
                    "out_polys": int(m.group("out_polys")),
                    "out_verts": int(m.group("out_verts")),
                    "conv_in_ns": int(m.group("conv_in")),
                    "ops_ns": int(m.group("ops")),
                    "conv_out_ns": int(m.group("conv_out")),
                }
            )
        elif '"event":"module_log"' in line and "T68-PROBE" in line:
            ev = json.loads(line)
            m = GUEST_RE.search(ev["message"])
            assert m, ev["message"]
            dumps += 1
            row = [int(v) for v in m.group(1).split(",")]
            cnt = [int(v) for v in m.group(2).split(",")]
            for i in range(12):
                guest_ns[i] += row[i]
                guest_calls[i] += cnt[i]
            last_layer_row = {
                "layer_index": ev.get("layer_index"),
                "ns_us": row,
                "calls": cnt,
            }
        elif '"event":"phase_complete"' in line:
            ev = json.loads(line)
            phases[ev["phase"]] = ev["elapsed_ms"]

    host_sites.sort(key=lambda r: r["seq"])
    guest_sites = [
        {
            "site": i,
            "name": SITE_NAMES.get(i, "?"),
            "calls": guest_calls[i],
            "guest_us": guest_ns[i],
        }
        for i in range(12)
    ]
    model = run_row["model"]
    layers = {"benchy": 240, "base": 495}[model]
    capture = {
        "metadata": {
            "run_id": run_id,
            "model": model,
            "generator": run_row["generator"],
            "supports": run_row["supports"],
            "layers": layers,
            "threads": int(run_row["threads"]),
            "wall_seconds": float(run_row["wall_seconds"]),
            "cpu_seconds": float(run_row["cpu_seconds"]),
            "bytes": int(run_row["bytes"]),
            "sha256": run_row["sha256"],
            "reference_sha256": run_row["reference_sha256"],
            "config_sha256": run_row["config_sha256"],
            "stderr_sha256": run_row["stderr_sha256"],
            "status": run_row["status"],
            "degraded": run_row["degraded"] == "True",
            "fatal": int(run_row["fatal"]),
            "non_fatal": int(run_row["non_fatal"]),
            "command": run_row["command"],
            "captured_commit": inputs["captured_commit"],
            "exe_sha256": inputs["exe_sha256"],
            "probe_guest_sha256": inputs["probe_guest_sha256"],
        },
        "phases_ms": phases,
        "host_sites": host_sites,
        "guest_sites": guest_sites,
        "guest_layer_dumps": dumps,
        "last_layer_dump": last_layer_row,
        "profile_host_native": parse_host_table(text, "host:native"),
    }
    return capture


def main() -> int:
    raw = (
        Path(sys.argv[1])
        if len(sys.argv) > 1
        else HERE.parents[4] / "target/t68-host-roundtrips"
    )
    inputs = json.loads((raw / "inputs.json").read_text(encoding="utf-8-sig"))
    with (raw / "runs.csv").open(encoding="utf-8-sig", newline="") as handle:
        runs = list(csv.DictReader(handle))

    captures = []
    for run_row in runs:
        captures.append(reduce_cell(raw, run_row, inputs))

    out_dir = HERE / "captures"
    out_dir.mkdir(exist_ok=True)
    for capture in captures:
        (out_dir / f"{capture['metadata']['run_id']}.json").write_text(
            json.dumps(capture, indent=1, sort_keys=True) + "\n", encoding="utf-8"
        )

    summary = {
        "captured_commit": inputs["captured_commit"],
        "exe_sha256": inputs["exe_sha256"],
        "probe_guest_sha256": inputs["probe_guest_sha256"],
        "cells": {c["metadata"]["run_id"]: c for c in captures},
    }
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )
    for c in captures:
        print(
            f"{c['metadata']['run_id']}: dumps={c['guest_layer_dumps']} "
            f"host_sites={len(c['host_sites'])} output={c['metadata']['sha256'][:12]}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
