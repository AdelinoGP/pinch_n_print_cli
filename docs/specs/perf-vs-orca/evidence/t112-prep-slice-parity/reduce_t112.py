#!/usr/bin/env python3
"""t112 prep-slice parity: reduce raw probe/profile captures into ``reduced/``.

Raw inputs (gitignored, produced by this take):

* ``target/t112/<fixture>.csv``        — the t112 probe scan (t59 probe harness
  plus the canonical arm), one row per layer with a non-empty flat family.
* ``target/t112/profile-<fixture>.jsonl`` — stderr capture of
  ``pnp_cli slice --profile --instrument-stderr`` for the supports-off classic
  cell of each fixture.

Outputs (committed):

* ``reduced/summary-<fixture>.json``   — probe aggregates + per-layer rows.
* ``reduced/profile-<fixture>.json``   — ``PrePass::Slice`` stage wall, native
  owner thread-time, and the ``polygon_ops::closing_ex`` scope row.
* ``reduced/ceilings.json``            — the eight-cell ceiling arithmetic.

    python docs/specs/perf-vs-orca/evidence/t112-prep-slice-parity/reduce_t112.py
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t112"
REDUCED = HERE / "reduced"

# Cell walls of record: PnP ordinary medians, t103 (supports-off) and t106
# (supports-on, post-pin). Ledger facts — re-derive from the two evidence
# files before quoting, never from this comment.
CELL_WALL_MS = {
    "benchy-classic-off": 7_680.0,
    "benchy-arachne-off": 7_690.0,
    "benchy-classic-on": 12_400.0,
    "benchy-arachne-on": 12_220.0,
    "base-classic-off": 75_380.0,
    "base-arachne-off": 78_890.0,
    "base-classic-on": 160_860.0,
    "base-arachne-on": 161_140.0,
}

FLOOR_FRACTION = 0.008


def reduce_probe(fixture: str) -> dict:
    path = RAW / f"{fixture}.csv"
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        columns = list(reader.fieldnames or [])
        rows = []
        for rec in reader:
            rows.append(rec)

    keys_int = (
        "layer",
        "n_comp",
        "prod_emitted",
        "open_emitted",
        "support_polys",
        "support_verts",
        "flat_verts",
        "differ",
    )
    keys_float = (
        "z",
        "prod_area",
        "flat_area",
        "open_area",
        "canon_area",
        "close_ms",
        "open_ms",
        "canon_ms",
    )

    parsed = []
    for rec in rows:
        out = {}
        for k in keys_int:
            out[k] = int(rec[k])
        for k in keys_float:
            out[k] = float(rec[k])
        parsed.append(out)

    def total(key: str) -> float:
        return sum(r[key] for r in parsed)

    calls = len(parsed)
    summary = {
        "fixture": fixture,
        "layers_columns": columns,
        "closing_calls": calls,
        "components_total": total("n_comp"),
        "prod_emitted_total": total("prod_emitted"),
        "open_components_total": total("open_emitted"),
        "prod_area_mm2": round(total("prod_area"), 4),
        "flat_area_mm2": round(total("flat_area"), 4),
        "open_area_mm2": round(total("open_area"), 4),
        "canon_area_mm2": round(total("canon_area"), 4),
        "close_ms_sum": round(total("close_ms"), 3),
        "open_ms_sum": round(total("open_ms"), 3),
        "canon_ms_sum": round(total("canon_ms"), 3),
        "differ_layers": sum(1 for r in parsed if r["differ"] == 1),
        "open_gt_prod_layers": sum(
            1 for r in parsed if r["open_area"] > r["prod_area"] + 1e-6
        ),
        "open_lt_prod_layers": sum(
            1 for r in parsed if r["open_area"] < r["prod_area"] - 1e-6
        ),
        "layers": parsed,
    }
    return summary


def read_capture(path: Path) -> dict:
    summary = None
    stage_wall = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (
            event.get("event") == "stage_complete"
            and event.get("stage") == "PrePass::Slice"
        ):
            stage_wall = max(int(event.get("elapsed_ms") or 0), stage_wall or 0)
        if event.get("event") == "profile_summary" and event.get("profile"):
            summary = event["profile"]
    if summary is None:
        raise SystemExit(f"no profile_summary in {path}")
    if stage_wall is None:
        raise SystemExit(f"no PrePass::Slice stage_complete in {path}")

    owner = None
    for module in summary["modules"]:
        if module["unit"] == "wall_ns" and module["module_id"] == "host:slice":
            owner = module
    if owner is None:
        raise SystemExit(f"no host:slice native row in {path}")

    closing = next(
        (s for s in owner["scopes"] if s["scope"] == "polygon_ops::closing_ex"), None
    )
    if closing is None:
        raise SystemExit(
            f"no polygon_ops::closing_ex row under PrePass::Slice in {path}"
        )

    return {
        "fixture": path.stem.split("-", 1)[1],
        "stage_wall_ms": stage_wall,
        "slice_thread_ms": round(owner["total_wall_ns"] / 1e6, 3),
        "slice_activations": owner["calls"],
        "closing_thread_ms": round(closing["self_wall_ns"] / 1e6, 3),
        "closing_calls": closing["calls"],
        "scopes": [
            {
                "scope": s["scope"],
                "self_wall_ms": round(s["self_wall_ns"] / 1e6, 3),
                "calls": s["calls"],
            }
            for s in owner["scopes"]
        ],
    }


def main() -> int:
    REDUCED.mkdir(parents=True, exist_ok=True)

    probes = {}
    for fixture in ("benchy", "base"):
        probes[fixture] = reduce_probe(fixture)
        (REDUCED / f"summary-{fixture}.json").write_text(
            json.dumps(probes[fixture], indent=1) + "\n",
            encoding="utf-8",
            newline="\n",
        )

    profiles = {}
    for fixture in ("benchy", "base"):
        profiles[fixture] = read_capture(RAW / f"profile-{fixture}.jsonl")
        (REDUCED / f"profile-{fixture}.json").write_text(
            json.dumps(profiles[fixture], indent=1) + "\n",
            encoding="utf-8",
            newline="\n",
        )

    cells = []
    for cell, wall_ms in CELL_WALL_MS.items():
        fixture = cell.split("-", 1)[0]
        probe = probes[fixture]
        profile = profiles[fixture]
        closing = profile["closing_thread_ms"]
        stage_wall = profile["stage_wall_ms"]
        stage_thread = profile["slice_thread_ms"]
        opening = probe["open_ms_sum"]
        gross = min(closing, stage_wall)
        net = gross - opening
        share = closing / stage_thread
        expected = share * stage_wall
        floor_ms = FLOOR_FRACTION * wall_ms
        cells.append(
            {
                "cell": cell,
                "fixture": fixture,
                "cell_wall_ms": wall_ms,
                "slice_stage_wall_ms": stage_wall,
                "slice_thread_ms": stage_thread,
                "closing_thread_ms": closing,
                "closing_share_of_slice_thread": round(share, 4),
                "expected_wall_ms_uniform_concurrency": round(expected, 3),
                "opening_thread_ms_p1": round(opening, 3),
                "gross_ceiling_ms": round(gross, 3),
                "net_ceiling_ms": round(net, 3),
                "net_ceiling_pct_of_cell": round(100.0 * net / wall_ms, 3),
                "floor_ms": round(floor_ms, 3),
                "floor_multiple": round(net / floor_ms, 2),
                "clears_floor": net >= floor_ms,
            }
        )

    payload = {
        "note": (
            "Net ceiling = min(closing_ex thread-time, PrePass::Slice stage wall) "
            "minus the measured canonical-opening thread-time (shape P1). Both "
            "terms are accumulated thread-time, so the result is an upper bound "
            "on the wall saving (parallelism >= 1), and subtracting the opening "
            "cost in full is conservative. Cell walls are the PnP ordinary "
            "medians of record (t103 supports-off, t106 supports-on)."
        ),
        "floor_fraction": FLOOR_FRACTION,
        "cells": cells,
    }
    (REDUCED / "ceilings.json").write_text(
        json.dumps(payload, indent=1) + "\n", encoding="utf-8", newline="\n"
    )
    print(json.dumps(payload, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
