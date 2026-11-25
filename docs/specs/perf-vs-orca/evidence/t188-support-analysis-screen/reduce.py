#!/usr/bin/env python3
"""Reduce raw t188 probe + screen captures under target/t188/ into committed
reduced JSONs under docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/reduced/.
"""
from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PROBE_RAW = REPO / "target" / "t188" / "probe" / "runs" / "probe_raw.json"
PROBE_DIR = REPO / "target" / "t188" / "probe" / "runs"
SCREEN_RAW = REPO / "target" / "t188" / "screen" / "runs" / "screen_raw.json"
SCREEN_DIR = REPO / "target" / "t188" / "screen" / "runs"
REDUCED = HERE / "reduced"

ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
OFF_CELLS = ["benchy-classic-off", "base-classic-off"]


def parse_stage_sa(stderr_path: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    sa_stage_ms = None
    if not stderr_path.exists():
        return {}
    with stderr_path.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"code":1200' in line or '"code": 1200' in line or "exact-Z occupancy" in line:
                code_1200 += 1
            s = line.strip()
            if not s.startswith("{"):
                continue
            try:
                obj = json.loads(s)
            except json.JSONDecodeError:
                continue
            ev = obj.get("event")
            if ev == "slice_complete":
                degraded = obj.get("degraded")
                fatal = obj.get("fatal_error_count")
                non_fatal = obj.get("non_fatal_error_count")
                slice_complete_ms = obj.get("elapsed_ms")
            elif ev == "stage_complete" and obj.get("stage") == "PrePass::SupportAnalysis":
                sa_stage_ms = obj.get("elapsed_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sa_stage_ms": sa_stage_ms,
    }


def parse_profile_sa_full(stderr_path: Path) -> dict:
    txt = stderr_path.read_text(encoding="utf-8", errors="replace")
    for line in txt.splitlines():
        s = line.strip()
        if s.startswith("{") and '"profile_summary"' in s:
            obj = json.loads(s)
            prof = obj.get("profile", {})
            sa_entry = None
            for m in prof.get("modules", []):
                if m.get("module_id") == "host:support_analysis":
                    sa_entry = m
                    break
            scopes = {}
            if sa_entry:
                for sc in sa_entry.get("scopes", []):
                    scopes[sc["scope"]] = {
                        "calls": sc["calls"],
                        "wall_ns": sc["total_wall_ns"],
                    }
            return {
                "fuel_total": prof.get("fuel_total"),
                "wall_total_ns": prof.get("wall_total_ns"),
                "sa_total_calls": sa_entry["calls"] if sa_entry else 0,
                "sa_total_wall_ns": sa_entry["total_wall_ns"] if sa_entry else 0,
                "scopes": scopes,
            }
    return {}


def reduce_probe() -> dict:
    raw = json.loads(PROBE_RAW.read_text(encoding="utf-8"))
    cells_out = {}
    for cell in ON_CELLS:
        probe_rows = [r for r in raw["rows"] if r["cell"] == cell and r["kind"] == "probe"]
        prof_row = next(r for r in raw["rows"] if r["cell"] == cell and r["kind"] == "profile")
        prof_stderr = PROBE_DIR / f"{cell}.profile.r1.stderr.txt"
        prof_parsed = parse_profile_sa_full(prof_stderr)

        passes = []
        for r in probe_rows:
            stderr_p = PROBE_DIR / f"{cell}.probe.r{r['round']}.stderr.txt"
            inst = parse_stage_sa(stderr_p)
            passes.append({
                "round": r["round"],
                "exit_code": r["exit_code"],
                "wall_s": r["wall_s"],
                "cpu_s": r["cpu_s"],
                "gcode_sha256": r["gcode_sha256"],
                "gcode_bytes": r["gcode_bytes"],
                "reference_match": r["reference_match"],
                "inst": inst,
                "total_wall_ns": r["probe"]["total_wall_ns"],
                "phases": r["probe"]["phases"],
                "counts": r["probe"]["counts"],
                "layer_summary": r["probe"]["layer_summary"],
                "ops": r["probe"]["ops"],
            })

        # Compute medians across the 3 probe passes
        phase_keys = list(passes[0]["phases"].keys())
        op_keys = sorted({k for p in passes for k in p["ops"].keys()})
        med_phases = {
            k: int(statistics.median(p["phases"][k] for p in passes)) for k in phase_keys
        }
        med_ops = {}
        for k in op_keys:
            med_ops[k] = {
                "ns": int(statistics.median(p["ops"][k]["ns"] for p in passes)),
                "max_ns": int(statistics.median(p["ops"][k]["max_ns"] for p in passes)),
                "count": passes[0]["ops"][k]["count"],
                "vin": passes[0]["ops"][k]["vin"],
                "vout": passes[0]["ops"][k]["vout"],
            }

        top_layers = probe_rows[0]["probe"]["top_layers"][:12]
        cells_out[cell] = {
            "passes": passes,
            "median": {
                "total_wall_ns": int(statistics.median(p["total_wall_ns"] for p in passes)),
                "sa_stage_ms": int(statistics.median(p["inst"]["sa_stage_ms"] for p in passes)),
                "slice_complete_ms": int(
                    statistics.median(p["inst"]["slice_complete_ms"] for p in passes)
                ),
                "phases": med_phases,
                "counts": passes[0]["counts"],
                "layer_summary": passes[0]["layer_summary"],
                "ops": med_ops,
            },
            "top_layers_r1": top_layers,
            "profile_pass": {
                "exit_code": prof_row["exit_code"],
                "gcode_sha256": prof_row["gcode_sha256"],
                "gcode_bytes": prof_row["gcode_bytes"],
                "reference_match": prof_row["reference_match"],
                "profile": prof_parsed,
            },
        }

    # Include geometry census empirical facts (separate diagnostic pass recorded in FINDINGS.md)
    geometry_census = {
        "benchy-classic-on": {
            "layers": 239,
            "lower_expolys": 580,
            "lower_contour_verts": 391000,
            "lower_holes": 175,
            "lower_hole_verts": 52688,
            "holes_lt_2delta": 0,
            "holes_lt_2delta_verts": 0,
            "holes_lt_6mm": 108,
            "holes_lt_6mm_verts": 32754,
            "holes_overlap_raw": 166,
            "holes_overlap_raw_verts": 52184,
            "raw_empty_layers": 0,
            "raw_empty_trimmed_nonempty_errors": 0,
            "raw_verts": 546499,
            "grown_orig_single_thread_ms": 339.7,
            "grown_cert_hits": 103,
            "grown_cert_errors": 0,
            "grown_cert_single_thread_ms": 880.7,
            "gholes_single_thread_ms": 360.2,
            "gholes_diff_layers": 2,
            "gwin_single_thread_ms": 400.2,
            "gwin_diff_layers": 2,
            "gisland_single_thread_ms": 419.9,
            "gisland_diff_layers": 2,
            "cantilever_layers": 81,
            "cantilever_real_true": 2,
            "cantilever_orig_single_thread_ms": 161.1,
            "cantilever_cert_hits": 79,
            "cantilever_cert_errors": 0,
            "cantilever_cert_single_thread_ms": 8.5,
        },
        "base-classic-on": {
            "layers": 495,
            "lower_expolys": 839,
            "lower_contour_verts": 2501360,
            "lower_holes": 48,
            "lower_hole_verts": 12874,
            "holes_lt_2delta": 20,
            "holes_lt_2delta_verts": 2283,
            "holes_lt_6mm": 47,
            "holes_lt_6mm_verts": 12054,
            "holes_overlap_raw": 41,
            "holes_overlap_raw_verts": 11053,
            "raw_empty_layers": 9,
            "raw_empty_trimmed_nonempty_errors": 0,
            "raw_verts": 2369902,
            "grown_orig_single_thread_ms": 5913.8,
            "grown_cert_hits": 50,
            "grown_cert_errors": 0,
            "grown_cert_single_thread_ms": 570.4,
            "gholes_single_thread_ms": 5877.1,
            "gholes_diff_layers": 41,
            "gwin_single_thread_ms": 6210.8,
            "gwin_diff_layers": 50,
            "gisland_single_thread_ms": 5678.2,
            "gisland_diff_layers": 121,
            "cantilever_layers": 368,
            "cantilever_real_true": 11,
            "cantilever_orig_single_thread_ms": 5613.5,
            "cantilever_cert_hits": 352,
            "cantilever_cert_errors": 0,
            "cantilever_cert_single_thread_ms": 83.4,
        },
    }

    return {
        "schema": "t188_probe_wall_v1",
        "captured_utc": raw["captured_utc"],
        "head_commit": "a5aea3ae",
        "rayon_num_threads": 12,
        "exe_sha256": raw["exe_sha256"],
        "total_runs": len(raw["rows"]),
        "cells": cells_out,
        "geometry_census": geometry_census,
    }


def reduce_screen() -> dict:
    raw = json.loads(SCREEN_RAW.read_text(encoding="utf-8"))
    supports_on = {}
    for cell in ON_CELLS:
        b_ab = [
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "ab"
        ]
        c_ab = [
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "ab"
        ]
        b_inst_r = next(
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "inst"
        )
        c_inst_r = next(
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "inst"
        )
        b_inst = parse_stage_sa(SCREEN_DIR / f"{cell}.baseline.inst.r1.stderr.txt")
        c_inst = parse_stage_sa(SCREEN_DIR / f"{cell}.candidate.inst.r1.stderr.txt")

        b_prof_r = next(
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "profile"
        )
        c_prof_r = next(
            r for r in raw["rows"] if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "profile"
        )

        pairs = []
        for rnd in (1, 2, 3):
            br = next(r for r in b_ab if r["round"] == rnd)
            cr = next(r for r in c_ab if r["round"] == rnd)
            pairs.append({
                "round": rnd,
                "baseline_wall_s": br["wall_s"],
                "candidate_wall_s": cr["wall_s"],
                "baseline_cpu_s": br["cpu_s"],
                "candidate_cpu_s": cr["cpu_s"],
                "baseline_sha256": br["gcode_sha256"],
                "candidate_sha256": cr["gcode_sha256"],
                "baseline_bytes": br["gcode_bytes"],
                "candidate_bytes": cr["gcode_bytes"],
                "reference_match": br["reference_match"] and cr["reference_match"],
            })

        supports_on[cell] = {
            "ab_pairs": pairs,
            "inst": {
                "baseline": {
                    "wall_s": b_inst_r["wall_s"],
                    "cpu_s": b_inst_r["cpu_s"],
                    "gcode_sha256": b_inst_r["gcode_sha256"],
                    "gcode_bytes": b_inst_r["gcode_bytes"],
                    "reference_match": b_inst_r["reference_match"],
                    **b_inst,
                },
                "candidate": {
                    "wall_s": c_inst_r["wall_s"],
                    "cpu_s": c_inst_r["cpu_s"],
                    "gcode_sha256": c_inst_r["gcode_sha256"],
                    "gcode_bytes": c_inst_r["gcode_bytes"],
                    "reference_match": c_inst_r["reference_match"],
                    **c_inst,
                },
            },
            "profile": {
                "baseline": {
                    "gcode_sha256": b_prof_r["gcode_sha256"],
                    "gcode_bytes": b_prof_r["gcode_bytes"],
                    "reference_match": b_prof_r["reference_match"],
                    **b_prof_r["profile"],
                },
                "candidate": {
                    "gcode_sha256": c_prof_r["gcode_sha256"],
                    "gcode_bytes": c_prof_r["gcode_bytes"],
                    "reference_match": c_prof_r["reference_match"],
                    **c_prof_r["profile"],
                },
            },
        }

    supports_off = {}
    for cell in OFF_CELLS:
        r = next(r for r in raw["rows"] if r["cell"] == cell and r["kind"] == "ab")
        supports_off[cell] = {
            "wall_s": r["wall_s"],
            "cpu_s": r["cpu_s"],
            "gcode_sha256": r["gcode_sha256"],
            "gcode_bytes": r["gcode_bytes"],
            "reference_match": r["reference_match"],
        }

    ablation = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        ab_rows = [r for r in raw["rows"] if r["cell"] == cell and r["kind"] == "ablation"]
        modes = {}
        for r in ab_rows:
            m = r["mode"]
            inst = parse_stage_sa(SCREEN_DIR / f"{cell}.candidate.{m}.ablation.r1.stderr.txt")
            modes[m] = {
                "wall_s": r["wall_s"],
                "cpu_s": r["cpu_s"],
                "gcode_sha256": r["gcode_sha256"],
                "gcode_bytes": r["gcode_bytes"],
                "reference_match": r["reference_match"],
                **inst,
            }
        ablation[cell] = modes

    return {
        "schema": "t188_screen_v1",
        "captured_utc": raw["captured_utc"],
        "head_commit": "a5aea3ae",
        "rayon_num_threads": 12,
        "baseline_exe_sha256": raw["baseline_exe_sha256"],
        "candidate_exe_sha256": raw["candidate_exe_sha256"],
        "bundle": "SA-1 + SA-2 + SA-3 + SA-4 + SA-5 + SA-6",
        "measured_runs": len(raw["rows"]),
        "warmup_runs": 2,
        "supports_on": supports_on,
        "supports_off_inertness": supports_off,
        "ablation": ablation,
        "rejected_mechanisms": [
            {
                "id": "G_holes",
                "reason": "Falsified: 99.5% of base vertices are outer contours (only 0.5% holes), zero speed gain (5,914 -> 5,877 ms single-thread), and Clipper2 vertex/island ordering drift on 2 benchy / 41 base layers.",
            },
            {
                "id": "G_win",
                "reason": "Falsified: raw overhang wraps full perimeter, slower than baseline (5,914 -> 6,211 ms base, 340 -> 400 ms benchy) with Clipper2 drift on 2 benchy / 50 base layers.",
            },
            {
                "id": "G_island_win",
                "reason": "Falsified: slower on benchy (340 -> 420 ms), negligible on base (5,914 -> 5,678 ms), and Clipper2 drift on 2 benchy / 121 base layers.",
            },
            {
                "id": "G_cert",
                "reason": "Rejected: exact distance certificate on grown_offset (delta ~ 0.33 mm) is slower than ClipperOffset on benchy (880.7 ms vs 339.7 ms) with only 50/495 hits on base.",
            },
        ],
    }


def main() -> int:
    REDUCED.mkdir(parents=True, exist_ok=True)
    probe = reduce_probe()
    screen = reduce_screen()
    (REDUCED / "probe_wall.json").write_text(json.dumps(probe, indent=2) + "\n", encoding="utf-8")
    (REDUCED / "screen.json").write_text(json.dumps(screen, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {REDUCED / 'probe_wall.json'} and {REDUCED / 'screen.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
