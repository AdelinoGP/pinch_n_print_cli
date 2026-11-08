#!/usr/bin/env python3
"""Reduce the t100 probe captures into the in-repo `summary.json`.

The raw captures live under the gitignored probe tree
(`.local-artifacts/perimeter-reference-preparation/t100-allocation-arena/probe/captures/`);
this reducer is the audit path from them to the committed reduced set, as the
map's evidence policy requires. `verify.py` then checks claims against
`summary.json` alone, so a plain checkout can verify every number here without
the raw tree.

Usage:
    python docs/specs/perf-vs-orca/evidence/t100-allocation-arena/extract.py
"""

from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
CAPTURES = (
    REPO
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t100-allocation-arena"
    / "probe"
    / "captures"
)

CELL_WALL_S = 164.803  # t91 kept ordinary arm, ordinary base classic supports-on
FLOOR_FRACTION = 0.008
REFERENCE_SHA = "22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d"
REFERENCE_BYTES = 42_507_912

NUM = r"(\d+)"
KVS = re.compile(r"(\w+)=([^\s]+)")

RUNS = {
    "t100-classic-census": "sys",
    "t100-classic-bulk": "sys",
    "t100-classic-bulk2": "sys",
    "t100-classic-mimalloc": "mimalloc",
}


def parse_key_values(line: str) -> dict[str, str]:
    return dict(KVS.findall(line))


def parse_capture(capture: Path) -> dict:
    record = json.loads(capture.with_suffix(".json").read_text())
    raw = capture.with_suffix(".jsonl").read_text(errors="replace")
    out: dict = {
        "id": record["id"],
        "cell": record["cell"],
        "allocator": RUNS.get(record["id"], "unknown"),
        "bulk_requested": record.get("bulk", ""),
        "wall_s": record["wall_s"],
        "cpu_s": record["cpu_s"],
        "cpu_wall": record["cpu_wall"],
        "sha256": record["sha256"],
        "bytes": record["bytes"],
        "degraded": record["degraded"],
        "exe_sha256": record["exe_sha"],
    }
    shape = census = None
    bulk: list[dict] = []
    copies: list[dict] = []
    for line in raw.splitlines():
        if not line.startswith("T100 "):
            continue
        kv = parse_key_values(line)
        if line.startswith("T100 SHAPE"):
            shape = {k: int(v) for k, v in kv.items() if v.isdigit()}
        elif line.startswith("T100 CENSUS"):
            census = {
                "layers": int(kv["layers"]),
                "drop_us": int(kv["drop_us"]),
                "deallocs": int(kv["deallocs"]),
                "dealloc_bytes": int(kv["dealloc_bytes"]),
                "allocs_in_window": int(kv["allocs"]),
                "alloc_bytes_in_window": int(kv["alloc_bytes"]),
                "hist": {
                    int(k): int(v)
                    for k, v in (p.split(":") for p in kv["hist"].split(","))
                },
                "samples": [
                    s.split(":")[1] for s in kv.get("samples", "").split("|") if s
                ],
            }
        elif line.startswith("T100 BULK"):
            parts = line.split()
            bulk.append(
                {
                    "order": parts[3],
                    "blocks": int(kv["blocks"]),
                    "bytes": int(kv["bytes"]),
                    "touch_us": int(kv["touch_us"]),
                    "free_us": int(kv["free_us"]),
                }
            )
        elif line.startswith("T100 COPY"):
            parts = line.split()
            copies.append(
                {
                    "order": parts[3],
                    "bytes": int(kv["bytes"]),
                    "copy_us": int(kv["copy_us"]),
                    "checksum": int(kv.get("checksum", "0")),
                }
            )
    out["shape"] = shape
    out["census"] = census
    out["bulk"] = bulk
    out["copy"] = copies
    return out


def main() -> int:
    runs = [parse_capture(p) for p in sorted(CAPTURES.glob("t100-classic-*.jsonl"))]
    if not runs:
        raise SystemExit(f"no captures under {CAPTURES}")

    system = [r for r in runs if r["allocator"] == "sys"]
    mimalloc = [r for r in runs if r["allocator"] == "mimalloc"]
    clean = [r for r in system if r["bulk_requested"] == ""][0]
    sampled = [r for r in system if r["shape"] is not None][0]
    mem = mimalloc[0]

    def census_hist_total(c: dict) -> int:
        return sum(c["hist"].values())

    bucket4 = clean["census"]["hist"].get(4, 0)
    blocks = clean["census"]["deallocs"]
    wall_us = CELL_WALL_S * 1_000_000
    floor_us = wall_us * FLOOR_FRACTION

    # Conservative bulk-free arm: the 495-block (one arena per layer) price from
    # the run that measured it without the sampling instrumentation.
    bulk_after = [b for b in sampled["bulk"] if b["order"] == "after"]
    bulk_1 = next(b for b in bulk_after if b["blocks"] == 1)
    bulk_495 = next(b for b in bulk_after if b["blocks"] == 495)
    bulk_free_us = min(bulk_1["free_us"], bulk_495["free_us"])
    bulk_free_conservative_us = max(bulk_1["free_us"], bulk_495["free_us"])
    ceiling_us = clean["census"]["drop_us"] - bulk_free_conservative_us

    copy_us = min(c["copy_us"] for c in sampled["copy"] if c["copy_us"] > 0)

    sampling_overhead_us = sampled["census"]["drop_us"] - clean["census"]["drop_us"]
    mimalloc_corrected_us = mem["census"]["drop_us"] - sampling_overhead_us
    mimalloc_saving_us = clean["census"]["drop_us"] - mimalloc_corrected_us

    geometry_blocks = (
        sampled["shape"]["entities"]
        + sampled["shape"]["layers"]
        + sampled["shape"]["annotations"]
        + sampled["shape"]["retracts"]
        + sampled["shape"]["travels"]
    )
    geometry_bytes = (
        sampled["shape"]["points"] * sampled["shape"]["size_point"]
        + sampled["shape"]["entities"] * sampled["shape"]["size_entity"]
        + sampled["shape"]["layers"] * sampled["shape"]["size_layer"]
    )
    rows = sampled["shape"]["supports"]
    demand_strings = bucket4 - 2 * rows

    summary = {
        "ticket": "100-allocation-volume-layer-collection",
        "take": "2026-10-30",
        "head": "cc2c54e6",
        "cell": "ordinary base classic supports-on",
        "cell_wall_s": CELL_WALL_S,
        "floor_fraction": FLOOR_FRACTION,
        "floor_us": round(floor_us, 1),
        "reference_sha256": REFERENCE_SHA,
        "reference_bytes": REFERENCE_BYTES,
        "method": {
            "census_window": "explicit drop(layer_irs) in DefaultGCodeEmitter::emit_gcode_owned",
            "allocator": "Counting<AccountingAllocator<System>> (probe build); Counting<MiMalloc> for the allocator arm",
            "bulk_arm": "alloc blocks summing to the censused byte volume, touch every page, time the free",
            "copy_arm": "sequential copy of the same byte volume (half read, half written), checksum read back",
            "sampling_note": "the sample instrumentation costs one extra atomic increment and an 8-byte copy per 8-15-byte block; the sampling-free census is run t100-classic-census",
        },
        "runs": runs,
        "derived": {
            "blocks": blocks,
            "bytes": clean["census"]["dealloc_bytes"],
            "ns_per_block": round(clean["census"]["drop_us"] * 1000 / blocks, 1),
            "bucket4_share": round(bucket4 / blocks, 6),
            "geometry_blocks": geometry_blocks,
            "geometry_block_share": round(geometry_blocks / blocks, 8),
            "geometry_bytes": geometry_bytes,
            "geometry_byte_share": round(
                geometry_bytes / clean["census"]["dealloc_bytes"], 6
            ),
            "support_rows": rows,
            "demand_strings": demand_strings,
            "demand_strings_per_row": round(demand_strings / rows, 1),
            "bulk_free_us": bulk_free_us,
            "bulk_free_conservative_us": bulk_free_conservative_us,
            "bulk_free_ceiling_us": ceiling_us,
            "bulk_free_ceiling_share": round(ceiling_us / wall_us, 5),
            "bulk_free_ceiling_multiple_of_floor": round(ceiling_us / floor_us, 2),
            "copy_us": copy_us,
            "flatten_total_us": copy_us + clean["census"]["drop_us"],
            "flatten_penalty_vs_baseline_us": copy_us,
            "sampling_overhead_us": sampling_overhead_us,
            "mimalloc_drop_measured_us": mem["census"]["drop_us"],
            "mimalloc_drop_corrected_us": mimalloc_corrected_us,
            "mimalloc_saving_measured_us": clean["census"]["drop_us"]
            - mem["census"]["drop_us"],
            "mimalloc_saving_corrected_us": mimalloc_saving_us,
            "mimalloc_saving_share_measured": round(
                (clean["census"]["drop_us"] - mem["census"]["drop_us"]) / wall_us, 5
            ),
            "mimalloc_saving_share_corrected": round(mimalloc_saving_us / wall_us, 5),
            "mimalloc_saving_multiple_of_floor_measured": round(
                (clean["census"]["drop_us"] - mem["census"]["drop_us"]) / floor_us, 2
            ),
            "mimalloc_saving_multiple_of_floor_corrected": round(
                mimalloc_saving_us / floor_us, 2
            ),
        },
    }
    out = HERE / "summary.json"
    out.write_text(json.dumps(summary, indent=2) + "\n")
    print(f"wrote {out} ({len(runs)} runs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
