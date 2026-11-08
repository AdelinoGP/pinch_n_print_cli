#!/usr/bin/env python3
"""Reduce the t101 probe captures into `summary.json`.

The captures live under
`.local-artifacts/perimeter-reference-preparation/t101-side-table/probe/captures/`
(gitignored). This script is the only path from them to the committed reduced
set, so a plain checkout can re-verify every claim with `verify.py`.

    python docs/specs/perf-vs-orca/evidence/t101-side-table/extract.py
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
    / "t101-side-table"
    / "probe"
    / "captures"
)

FLOOR_US = 1_318_000  # 0.80 % of t91's kept ordinary base-classic-on wall (164.803 s)
CELL_WALL_US = 164_803_000

ASSEMBLE_RE = re.compile(r"^T101 ASSEMBLE layer=\d+ us=(\d+)$")
SHAPE_RE = re.compile(r"^T101 SHAPE\s+(\S+)\s+(.*)$")
CENSUS_RE = re.compile(r"^T101 CENSUS\s+(\S+)\s+(.*)$")
KV_RE = re.compile(r"(\w+)=([^\s]+)")


def kv(rest: str) -> dict[str, str]:
    return dict(KV_RE.findall(rest))


def parse_capture(path: Path) -> dict:
    record = json.loads(path.read_text())
    shape = None
    census = None
    assemble_us = []
    for line in record.get("probe", []):
        m = ASSEMBLE_RE.match(line)
        if m:
            assemble_us.append(int(m.group(1)))
            continue
        m = SHAPE_RE.match(line)
        if m:
            shape = kv(m.group(2))
            continue
        m = CENSUS_RE.match(line)
        if m:
            census = kv(m.group(2))
    if not assemble_us or shape is None or census is None:
        raise SystemExit(f"{path.name}: incomplete probe capture")
    shape_ints = {
        k: int(v)
        for k, v in shape.items()
        if k
        in {
            "layers",
            "entities",
            "rows",
            "ids",
            "id_bytes",
            "row_len_min",
            "row_len_p50",
            "row_len_p90",
            "row_len_max",
            "empty_lists",
            "bodies",
            "body_rows_p50",
            "body_rows_max",
            "mixed_bodies",
            "distinct_lists",
            "retained_ids",
            "retained_id_bytes",
            "present_blocks",
            "retained_blocks",
            "present_heap_bytes",
            "retained_heap_bytes",
        }
    }
    sampling_free = census.get("sampling_free") == "true"
    return {
        "id": record["id"],
        "skip_table": record["skip_table"],
        "census_enabled": record.get("census", True),
        "sampling_free": sampling_free,
        "wall_s": record["wall_s"],
        "cpu_s": record["cpu_s"],
        "sha256": record["sha256"],
        "bytes": record["bytes"],
        "degraded": record["degraded"],
        "exe_sha": record["exe_sha"],
        "assemble_calls": len(assemble_us),
        "assemble_us": sum(assemble_us),
        "shape": shape_ints,
        "census": {
            "drop_us": int(census["drop_us"]),
            "deallocs": None if sampling_free else int(census["deallocs"]),
            "dealloc_bytes": None if sampling_free else int(census["dealloc_bytes"]),
            "allocs_in_window": None if sampling_free else int(census["allocs"]),
            "alloc_bytes_in_window": None
            if sampling_free
            else int(census["alloc_bytes"]),
        },
    }


def median(values: list[float]) -> float:
    ordered = sorted(values)
    return ordered[len(ordered) // 2]


def main() -> int:
    runs = [parse_capture(p) for p in sorted(CAPTURES.glob("*.json"))]
    runs.sort(key=lambda r: r["id"])
    table_runs = [r for r in runs if not r["skip_table"]]
    skip_runs = [r for r in runs if r["skip_table"]]
    if not table_runs or not skip_runs:
        raise SystemExit("both arms are required")
    clean_table = [r for r in table_runs if r["sampling_free"]]
    # The skip arm frees ~63 k blocks, so its census overhead is immaterial
    # (~1 ms at the measured per-block rate) and every skip arm is used.
    clean_skip = skip_runs
    if not clean_table:
        raise SystemExit("sampling-free table drop arms are required for the ceiling")

    # Every table arm prints the same shape; take the last one and assert it.
    shapes = {json.dumps(r["shape"], sort_keys=True) for r in table_runs}
    if len(shapes) != 1:
        raise SystemExit("shape census is not deterministic across table arms")
    shape = table_runs[-1]["shape"]

    asm_table = [r["assemble_us"] for r in table_runs]
    asm_skip = [r["assemble_us"] for r in skip_runs]
    drop_table = [r["census"]["drop_us"] for r in clean_table]
    drop_skip = [r["census"]["drop_us"] for r in clean_skip]
    censused_table = [r for r in table_runs if not r["sampling_free"]]

    ceiling_uncontended = (min(asm_table) - max(asm_skip)) + (
        min(drop_table) - max(drop_skip)
    )
    ceiling_generous = (max(asm_table) - min(asm_skip)) + (
        max(drop_table) - min(drop_skip)
    )

    summary = {
        "note": (
            "Reduced t101 Phase-1 screening set. Derived by extract.py from the "
            "gitignored probe captures; verify.py reads only this file."
        ),
        "cell": "ordinary base classic supports-on",
        "fixture": "tmp/base.stl (gitignored, licence-encumbered)",
        "config": "evidence/matched-pair/configs/pnp-classic-supports-on.json",
        "reference_sha256": "22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d",
        "reference_bytes": 42_507_912,
        "cell_wall_us": CELL_WALL_US,
        "floor_us": FLOOR_US,
        "t100_system_heap_drop_us": 3_526_544,
        "runs": runs,
        "shape": shape,
        "derived": {
            "table_runs": len(table_runs),
            "skip_runs": len(skip_runs),
            "clean_table_runs": len(clean_table),
            "clean_skip_runs": len(clean_skip),
            "rows_per_body": shape["rows"] / shape["bodies"],
            "retained_block_share": shape["retained_blocks"] / shape["present_blocks"],
            "retained_heap_share": shape["retained_heap_bytes"]
            / shape["present_heap_bytes"],
            "median_assemble_table_us": median(asm_table),
            "median_assemble_skip_us": median(asm_skip),
            "median_drop_table_us": median(drop_table),
            "median_drop_skip_us": median(drop_skip),
            "median_drop_table_censused_us": median(
                [r["census"]["drop_us"] for r in censused_table]
            )
            if censused_table
            else None,
            "census_overhead_us": (
                median([r["census"]["drop_us"] for r in censused_table])
                - median(drop_table)
            )
            if censused_table
            else None,
            "ceiling_uncontended_us": ceiling_uncontended,
            "ceiling_generous_us": ceiling_generous,
            "ceiling_uncontended_share": ceiling_uncontended / CELL_WALL_US,
            "ceiling_generous_share": ceiling_generous / CELL_WALL_US,
            "ceiling_uncontended_multiple": ceiling_uncontended / FLOOR_US,
            "ceiling_generous_multiple": ceiling_generous / FLOOR_US,
            "wall_median_table_s": median([r["wall_s"] for r in table_runs]),
            "wall_median_skip_s": median([r["wall_s"] for r in skip_runs]),
            "cpu_median_table_s": median([r["cpu_s"] for r in table_runs]),
            "cpu_median_skip_s": median([r["cpu_s"] for r in skip_runs]),
            "t100_over_t101_drop": 3_526_544 / median(drop_table),
        },
    }
    (HERE / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(
        f"wrote summary.json: {len(runs)} runs, "
        f"ceiling uncontended {ceiling_uncontended / 1000:.1f} ms "
        f"({ceiling_uncontended / FLOOR_US:.2f}x floor), "
        f"generous {ceiling_generous / 1000:.1f} ms "
        f"({ceiling_generous / FLOOR_US:.2f}x floor)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
