#!/usr/bin/env python3
"""t59 flat-bridge closing probe reducer.

Reads the gitignored raw captures under ``target/t59/`` produced by the
temporary probe harness (``probe.patch``):

- ``benchy.csv`` / ``base.csv`` — one row per layer that reached the
  enclosure closing in ``assemble_flat_bridge_areas``;
- ``benchy-161-drill.txt`` / ``base-8-drill.txt`` — the per-layer mechanism
  drill outputs.

Writes the in-repo reduced set under ``reduced/``:

- ``summary-<fixture>.json`` — aggregates + trimmed per-layer rows;
- ``mechanism.json`` — parsed drill measurements for the two drilled layers.

Re-derivation from a plain checkout (no raw tree):

    python docs/specs/perf-vs-orca/evidence/t59-flat-bridge-closing/reduce_t59.py

requires the raw captures; without them this script exits 1. The reduced set
alone is verified by ``verify.py``.
"""

from __future__ import annotations

import csv
import json
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
RAW = ROOT / "target" / "t59"
OUT = Path(__file__).resolve().parent / "reduced"


def _fail(msg: str) -> None:
    print(f"reduce_t59: {msg}", file=sys.stderr)
    sys.exit(1)


def read_layers(fixture: str) -> list[dict]:
    path = RAW / f"{fixture}.csv"
    if not path.exists():
        _fail(f"raw capture missing: {path} (run the probe harness first)")
    rows = list(csv.DictReader(path.open(encoding="utf-8-sig", newline="")))
    out = []
    for r in rows:
        out.append(
            {
                "layer": int(r["layer"]),
                "n_comp": int(r["n_comp"]),
                "support_polys": int(r["support_polys"]),
                "support_verts": int(r["support_verts"]),
                "global_ms": float(r["global_close_ms"]),
                "local_ms": float(r["local_close_ms"]),
                "clip_ms": float(r["clip_ms"]),
                "box_ratio": float(r["box_area_ratio"]),
                "n_near": int(r["n_near"]),
                "n_far": int(r["n_far"]),
                "base_emitted": int(r["base_emitted"]),
                "cand_emitted": int(r["cand_emitted"]),
                "verdict_ok": r["verdict_ok"] == "true",
                "min_margin": float(r["min_margin"]),
            }
        )
    return out


def summarize(fixture: str, rows: list[dict]) -> dict:
    calls = len(rows)
    mismatch = [r["layer"] for r in rows if not r["verdict_ok"]]
    sp = sorted(r["support_polys"] for r in rows)
    sv = sorted(r["support_verts"] for r in rows)
    nc = sorted(r["n_comp"] for r in rows)
    gm = sorted(r["global_ms"] for r in rows)
    lm = sorted(r["local_ms"] for r in rows)
    br = sorted(r["box_ratio"] for r in rows)
    g_sum = sum(r["global_ms"] for r in rows)
    l_sum = sum(r["local_ms"] for r in rows)
    clip_sum = sum(r["clip_ms"] for r in rows)
    if clip_sum + l_sum <= 0:
        _fail("degenerate local timings")
    return {
        "fixture": fixture,
        "closing_calls": calls,
        "verdict_ok_layers": calls - len(mismatch),
        "mismatch_count": len(mismatch),
        "mismatch_layers": mismatch,
        "global_closing_ms_sum": round(g_sum, 1),
        "local_closing_ms_sum": round(l_sum, 1),
        "local_clip_ms_sum": round(clip_sum, 1),
        "local_total_over_global": round((l_sum + clip_sum) / g_sum, 5),
        "global_ms_median": round(statistics.median(gm), 1),
        "local_ms_median": round(statistics.median(lm), 1),
        "components_total": sum(r["n_comp"] for r in rows),
        "emitted_base_total": sum(r["base_emitted"] for r in rows),
        "emitted_cand_total": sum(r["cand_emitted"] for r in rows),
        "far_components": sum(r["n_far"] for r in rows),
        "far_layers": sum(
            1 for r in rows if r["n_far"] == r["n_comp"] and r["n_comp"] > 0
        ),
        "min_margin": min(r["min_margin"] for r in rows),
        "near_components": sum(r["n_near"] for r in rows),
        "box_ratio_mean": round(sum(br) / len(br), 5),
        "box_ratio_median": round(statistics.median(br), 5),
        "support_polys_median": statistics.median(sp),
        "support_polys_p90": sp[int(0.9 * len(sp))],
        "support_polys_max": max(sp),
        "support_verts_median": statistics.median(sv),
        "support_verts_p90": sv[int(0.9 * len(sv))],
        "support_verts_max": max(sv),
        "n_comp_median": statistics.median(nc),
        "n_comp_max": max(nc),
        "layers": [
            [
                r["layer"],
                r["n_comp"],
                r["support_polys"],
                r["support_verts"],
                round(r["global_ms"], 1),
                round(r["local_ms"], 1),
                round(r["box_ratio"], 4),
                r["n_near"],
                r["n_far"],
                r["base_emitted"],
                r["cand_emitted"],
                1 if r["verdict_ok"] else 0,
                r["min_margin"],
            ]
            for r in rows
        ],
        "layers_columns": [
            "layer",
            "n_comp",
            "support_polys",
            "support_verts",
            "global_ms",
            "local_ms",
            "box_ratio",
            "n_near",
            "n_far",
            "base_emitted",
            "cand_emitted",
            "verdict_ok",
            "min_margin",
        ],
    }


NUM = r"([0-9.]+)"


def drill(path: Path) -> dict:
    if not path.exists():
        _fail(f"raw drill missing: {path}")
    text = path.read_text(encoding="utf-8")
    d: dict = {}
    d["fidelity_ok"] = "FIDELITY OK" in text

    def grab(pattern: str, key: str, cast=float):
        m = re.search(pattern, text)
        if not m:
            _fail(f"drill pattern missing ({key}): {pattern}")
        d[key] = cast(m.group(1))

    grab(rf"LAYER (\d+)", "layer", int)
    grab(rf"flat_bridge: n=(\d+) area_mm2={NUM}", "flat_bridge_n", int)
    grab(rf"flat_bridge: n=\d+ area_mm2={NUM}", "flat_bridge_area_mm2")
    grab(rf"pad_mm={NUM}", "pad_mm")
    grab(rf"support: n=(\d+)", "support_n", int)
    grab(rf"support: n=\d+ verts=(\d+)", "support_verts", int)
    grab(rf"support: n=\d+ verts=\d+ area_mm2={NUM}", "support_area_mm2")
    grab(rf"closed: n=\d+ area_mm2={NUM}", "closed_raw_mm2")
    grab(rf"refilled: n=(\d+) area_mm2={NUM}", "refilled_n", int)
    grab(rf"refilled: n=\d+ area_mm2={NUM}", "refilled_raw_mm2")
    grab(rf"local_support: n=(\d+)", "local_support_n", int)
    grab(rf"local_closed: n=\d+ area_mm2={NUM}", "closed_local_mm2")
    grab(rf"local_refilled: n=\d+ area_mm2={NUM}", "refilled_local_mm2")
    grab(rf"closed_again area_mm2={NUM}", "closed_raw_determinism_mm2")
    grab(rf"support_union: n=(\d+) verts=(\d+) area_mm2={NUM}", "support_union_n", int)
    grab(rf"support_union: n=\d+ verts=(\d+)", "support_union_verts", int)
    grab(rf"support_union: n=\d+ verts=\d+ area_mm2={NUM}", "support_union_area_mm2")
    grab(rf"closed_union area_mm2={NUM}", "closed_union_mm2")
    grab(r"support pieces: n=(\d+)", "pieces_n", int)
    grab(r"support pieces: n=\d+ tiny\(<0\.001mm2\)=(\d+)", "pieces_tiny", int)
    grab(r"support pieces: n=\d+ tiny\(<0\.001mm2\)=\d+ zero=(\d+)", "pieces_zero", int)
    grab(rf"sliver population: count=(\d+)", "slivers", int)
    grab(rf"closed_big_only area_mm2={NUM}", "closed_big_only_mm2")

    comps = []
    for m in re.finditer(
        r"comp (\d+): area_mm2=([0-9.]+) bbox_mm=\[[^]]+\]\.\.\[[^]]+\] frac_base=(-?[0-9.]+) frac_cand=(-?[0-9.]+)",
        text,
    ):
        comps.append(
            {
                "component": int(m.group(1)),
                "area_mm2": float(m.group(2)),
                "base_frac": float(m.group(3)),
                "cand_frac": float(m.group(4)),
            }
        )
    d["component_flips"] = comps
    d["flip_count"] = len(comps)
    return d


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for fixture in ("benchy", "base"):
        rows = read_layers(fixture)
        summary = summarize(fixture, rows)
        (OUT / f"summary-{fixture}.json").write_text(
            json.dumps(summary, indent=1) + "\n", encoding="utf-8"
        )
        print(f"wrote summary-{fixture}.json ({summary['closing_calls']} calls)")
    mech = {
        "note": (
            "Per-layer mechanism drills. 'closed_raw' is closing_ex over the "
            "production support path list; 'closed_union'/'closed_local' are the "
            "same point set re-emitted by union_ex / intersection (normalized "
            "path lists). The artifact field of the raw path list is what the "
            "10% gate consumes."
        ),
        "benchy_layer_161": drill(RAW / "benchy-161-drill.txt"),
        "base_layer_8": drill(RAW / "base-8-drill.txt"),
    }
    (OUT / "mechanism.json").write_text(
        json.dumps(mech, indent=1) + "\n", encoding="utf-8"
    )
    print("wrote mechanism.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
