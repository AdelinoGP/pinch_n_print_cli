#!/usr/bin/env python3
"""Reduce matched-pair scoreboard CSVs into the t124 committed evidence.

Accepts one or more rig CSVs (`--rows` is repeatable). A cell whose roster is
incomplete in any batch (e.g. a campaign interrupted mid-round) is discarded as
a whole and named in `discarded_partial_cells`; the same cell may be supplied
complete by a later batch. The merged, complete-only rows are written to
`scoreboard.csv`, so the committed evidence is one self-contained 96-row file.

Applies the map's standing starvation rule (a measured sample is excluded when
its cpu/wall ratio is below 0.75x its group's best ratio), takes medians over the
retained samples, and records the per-cell Orca ratios. PnP arms must be
byte-identical within a cell; Orca's G-code hash is not an identity (its
appendix embeds timing), so the Orca hash recorded here is the median sample's.

Usage:
    python reduce_t124.py --rows target/matched-pair/<batch>/scoreboard-rows.csv \
        [--rows target/matched-pair/<batch2>/scoreboard-rows.csv ...] \
        --out-dir docs/specs/perf-vs-orca/evidence/t124-post-t123-scoreboard
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

TOOLS = ("orca", "pnp-ordinary", "pnp-accelerated")
PNP_TOOLS = ("pnp-ordinary", "pnp-accelerated")
STARVATION_RATIO = 0.75
EXPECTED_CELLS = 8


def median(values: list[float]) -> float:
    return statistics.median(values)


def median_sample(samples: list[dict]) -> dict:
    ordered = sorted(samples, key=lambda r: float(r["wall_seconds"]))
    return ordered[(len(ordered) - 1) // 2]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rows", required=True, action="append", type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument(
        "--allow-partial",
        action="store_true",
        help="write fewer than 8 complete cells (campaign preview only; the committed artifact is built without this)",
    )
    args = parser.parse_args()

    rows: list[dict] = []
    fieldnames: list[str] = []
    batches: list[str] = []
    for path in args.rows:
        with path.open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            if not fieldnames:
                fieldnames = list(reader.fieldnames or [])
            elif list(reader.fieldnames or []) != fieldnames:
                raise SystemExit(f"{path}: CSV header differs from the first input")
            chunk = list(reader)
        rows.extend(chunk)
        batch = chunk[0]["batch_id"] if chunk else path.parent.name
        if batch not in batches:
            batches.append(batch)

    # Group by (batch, cell) so an interrupted cell is discarded as a whole and
    # a later batch can supply the same cell complete. A cell must be complete
    # in exactly one batch; merging partial runs from two batches would forge a
    # roster that no continuous window ever produced.
    by_batch_cell: dict[tuple[str, str, str, str], list[dict]] = defaultdict(list)
    for row in rows:
        by_batch_cell[
            (row["batch_id"], row["fixture"], row["generator"], row["supports"])
        ].append(row)

    complete_rows: list[dict] = []
    discarded: list[str] = []
    cells: dict[str, dict] = {}
    lines: list[str] = []
    lines.append(f"input rows={len(rows)} batches={','.join(batches)}")

    for cell in sorted({key[1:] for key in by_batch_cell}):
        fixture, generator, supports = cell
        key = f"{fixture}/{generator}/{supports}"
        candidates: list[tuple[str, list[dict]]] = []
        partials: list[str] = []
        for (batch, *batch_cell), cell_rows in sorted(by_batch_cell.items()):
            if tuple(batch_cell) != cell:
                continue
            groups: dict[str, list[dict]] = defaultdict(list)
            for row in cell_rows:
                groups[row["tool"]].append(row)
            rosters = {
                tool: (
                    len([r for r in groups.get(tool, []) if r["warmup"] != "0"]),
                    len([r for r in groups.get(tool, []) if r["warmup"] == "0"]),
                )
                for tool in TOOLS
            }
            if all(roster == (1, 3) for roster in rosters.values()):
                candidates.append((batch, cell_rows))
            else:
                partials.append(f"{batch}:{key} ({rosters})")
        if not candidates:
            discarded.extend(partials or [f"{key} (absent)"])
            continue
        if len(candidates) > 1:
            raise SystemExit(
                f"{key}: complete in more than one batch: {[b for b, _ in candidates]}"
            )
        discarded.extend(partials)
        _, cell_rows = candidates[0]
        groups = defaultdict(list)
        for row in cell_rows:
            groups[row["tool"]].append(row)

        summary: dict[str, dict] = {}
        raw_walls: dict[str, float] = {}
        lines.append("")
        lines.append(f"## {fixture} | {generator}-{supports}")
        for tool in TOOLS:
            measured = [r for r in groups[tool] if r["warmup"] == "0"]
            ratios = [
                float(r["cpu_seconds"]) / float(r["wall_seconds"]) for r in measured
            ]
            best = max(ratios)
            retained = [
                row
                for row, ratio in zip(measured, ratios)
                if ratio >= STARVATION_RATIO * best
            ]
            if not retained:
                retained = measured
                lines.append("    (all samples starvation-flagged; medians over all)")
            if tool in PNP_TOOLS:
                hashes = {r["sha256_of_gcode"] for r in measured}
                if len(hashes) != 1:
                    raise SystemExit(
                        f"{key}/{tool}: measured samples are not byte-identical"
                    )

            for row, ratio in zip(measured, ratios):
                flag = "  <-- STARVED, excluded" if row not in retained else ""
                lines.append(
                    f"    run{row['run_index']}: wall={float(row['wall_seconds']):.2f}s "
                    f"cpu={float(row['cpu_seconds']):.2f}s ratio={float(row['cpu_wall_ratio']):.4f}{flag}"
                )
            walls = [float(r["wall_seconds"]) for r in retained]
            cpus = [float(r["cpu_seconds"]) for r in retained]
            bytes_ = sorted({int(r["gcode_bytes"]) for r in measured})
            degraded = "/".join(sorted({r["degraded"] for r in measured}))
            non_fatal = "/".join(sorted({r["non_fatal_error_count"] for r in measured}))
            lines.append(
                f"    medians(n={len(retained)}): wall={median(walls):.2f}s cpu={median(cpus):.2f}s"
            )
            lines.append(f"    gcode bytes: {'/'.join(str(b) for b in bytes_)}")
            lines.append(f"    degraded={degraded} non_fatal={non_fatal}")

            recorded = median_sample(retained)
            med_wall = median(walls)
            summary[tool] = {
                "wall_s": round(med_wall, 4),
                "cpu_s": round(median(cpus), 4),
                "n": len(retained),
                "excluded": len(measured) - len(retained),
                "sha256": recorded["sha256_of_gcode"],
                "batch_id": recorded["batch_id"],
            }
            raw_walls[tool] = med_wall
        summary["ordinary_ratio"] = round(
            raw_walls["pnp-ordinary"] / raw_walls["orca"], 6
        )
        summary["accelerated_ratio"] = round(
            raw_walls["pnp-accelerated"] / raw_walls["orca"], 6
        )
        cells[key] = summary
        complete_rows.extend(cell_rows)

    if len(cells) != EXPECTED_CELLS and not args.allow_partial:
        raise SystemExit(
            f"expected {EXPECTED_CELLS} complete cells, built {len(cells)}: {sorted(cells)}"
        )
    if discarded:
        lines.append("")
        lines.append("## Discarded partial sources")
        for entry in discarded:
            lines.append(f"    {entry}")

    lines.append("")
    lines.append(
        f"## Committed set: {len(cells)} cells / {len(complete_rows)} rows "
        "(one complete batch per cell)"
    )

    lines.append("")
    lines.append("## Per-cell gaps (median wall ratios)")
    for key in sorted(cells):
        cell = cells[key]
        ordinary = cell["pnp-ordinary"]["wall_s"] / cell["orca"]["wall_s"]
        accelerated = cell["pnp-accelerated"]["wall_s"] / cell["orca"]["wall_s"]
        lines.append(
            f"{key}: pnp-ordinary/orca={ordinary:.2f}x  "
            f"pnp-accelerated/orca={accelerated:.2f}x  "
            f"accelerated/ordinary={accelerated / ordinary:.3f}"
        )

    summary_doc = {
        "ticket": "perf-vs-orca/124",
        "batches": batches,
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "starvation_ratio": STARVATION_RATIO,
        "row_count": len(complete_rows),
        "discarded_partial_sources": discarded,
        "cells": cells,
    }

    args.out_dir.mkdir(parents=True, exist_ok=True)
    with (args.out_dir / "scoreboard.csv").open(
        "w", newline="", encoding="utf-8"
    ) as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(complete_rows)
    (args.out_dir / "scoreboard-summary.json").write_text(
        json.dumps(summary_doc, indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )
    (args.out_dir / "scoreboard.txt").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )
    print(
        f"wrote {len(cells)} cells / {len(complete_rows)} rows to {args.out_dir}"
        + (f"; discarded partial: {discarded}" if discarded else "")
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
