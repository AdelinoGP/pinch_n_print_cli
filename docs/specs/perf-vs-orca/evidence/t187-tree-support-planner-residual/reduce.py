#!/usr/bin/env python3
"""Reduce raw t187 probe and screen captures from target/ into committed reduced/*.json.

Reads gitignored raw outputs under:
  - target/t187-probe/
  - target/t187-screen/runs/
  - target/t187-screen/p5test_*.gcode

Writes committed reduced evidence under:
  - docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/reduced/
      probe_wall.json
      probe_fuel.json
      screen.json
"""

from __future__ import annotations

import hashlib
import json
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
EVIDENCE_DIR = Path(__file__).resolve().parent
REDUCED_DIR = EVIDENCE_DIR / "reduced"
PROBE_DIR = ROOT / "target" / "t187-probe"
SCREEN_DIR = ROOT / "target" / "t187-screen"
SCREEN_RUNS_DIR = SCREEN_DIR / "runs"
REF_PATH = (
    ROOT
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t182-spatial-hierarchical-phase3-union"
    / "reference.json"
)

SUPPORTS_ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def load_reference() -> dict[str, dict]:
    raw = load_json(REF_PATH)
    active = raw["active_set"]
    sizes = raw.get("bytes", {})
    return {
        cell: {"sha256": sha, "bytes": sizes.get(cell)}
        for cell, sha in active.items()
    }


def parse_inst_stderr(path: Path) -> dict:
    """Parse --instrument-stderr JSONL events accurately using actual schema fields."""
    degraded = None
    fatal_error_count = None
    non_fatal_error_count = None
    slice_complete_ms = None
    code_1200_count = 0
    stages_ms: dict[str, int] = {}
    phases_ms: dict[str, int] = {}
    sg_modules_ms: dict[str, int] = {}

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        ev = obj.get("event")
        if ev == "slice_complete":
            degraded = obj.get("degraded")
            fatal_error_count = obj.get("fatal_error_count")
            non_fatal_error_count = obj.get("non_fatal_error_count")
            slice_complete_ms = obj.get("elapsed_ms")
        elif ev == "stage_complete":
            st = obj.get("stage")
            ms = obj.get("elapsed_ms")
            if isinstance(st, str) and isinstance(ms, (int, float)):
                stages_ms[st] = int(ms)
        elif ev == "phase_complete":
            ph = obj.get("phase")
            ms = obj.get("elapsed_ms")
            if isinstance(ph, str) and isinstance(ms, (int, float)):
                phases_ms[ph] = int(ms)
        elif ev == "module_complete":
            if obj.get("stage") == "PrePass::SupportGeometry":
                mid = obj.get("module_id")
                ms = obj.get("elapsed_ms")
                if isinstance(mid, str) and isinstance(ms, (int, float)):
                    sg_modules_ms[mid] = int(ms)
        elif ev in ("module_log", "diagnostic"):
            msg = str(obj.get("message", ""))
            code = obj.get("code")
            if code == 1200 or "code 1200" in msg or "1200:" in msg:
                code_1200_count += 1

    return {
        "degraded": degraded,
        "fatal_error_count": fatal_error_count,
        "non_fatal_error_count": non_fatal_error_count,
        "code_1200_count": code_1200_count,
        "slice_complete_ms": slice_complete_ms,
        "prepass_ms": phases_ms.get("prepass"),
        "per_layer_ms": phases_ms.get("per_layer"),
        "postpass_ms": phases_ms.get("postpass"),
        "support_geometry_stage_ms": stages_ms.get("PrePass::SupportGeometry"),
        "support_analysis_stage_ms": stages_ms.get("PrePass::SupportAnalysis"),
        "tree_support_planner_ms": sg_modules_ms.get("com.core.tree-support-planner"),
        "traditional_support_planner_ms": sg_modules_ms.get(
            "com.core.traditional-support-planner"
        ),
    }


def parse_profile_stderr(path: Path) -> dict:
    """Parse --profile JSONL profile_summary event."""
    prof = None
    slice_complete_ms = None
    degraded = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        ev = obj.get("event")
        if ev == "profile_summary":
            prof = obj.get("profile")
        elif ev == "slice_complete":
            slice_complete_ms = obj.get("elapsed_ms")
            degraded = obj.get("degraded")

    if prof is None:
        raise RuntimeError(f"Missing profile_summary in {path}")

    mods = {m["module_id"]: m for m in prof.get("modules", [])}

    def mod_fuel(mid: str) -> dict | None:
        m = mods.get(mid)
        if not m:
            return None
        return {
            "calls": m["calls"],
            "total_fuel": m["total_fuel"],
            "self_fuel": m["self_fuel"],
        }

    def host_builtin(mid: str) -> dict | None:
        m = mods.get(mid)
        if not m:
            return None
        scopes = {}
        for s in m.get("scopes", []):
            scopes[s["scope"]] = {
                "calls": s["calls"],
                "total_wall_ns": s["total_wall_ns"],
            }
        return {
            "calls": m["calls"],
            "accumulated_thread_wall_ns": m["total_wall_ns"],
            "scopes": scopes,
        }

    return {
        "slice_complete_ms": slice_complete_ms,
        "degraded": degraded,
        "fuel_total": prof["fuel_total"],
        "host_accumulated_thread_wall_total_ns": prof["wall_total_ns"],
        "tree_support_planner": mod_fuel("com.core.tree-support-planner"),
        "traditional_support_planner": mod_fuel("com.core.traditional-support-planner"),
        "host_native": host_builtin("host:native"),
        "host_support_analysis": host_builtin("host:support_analysis"),
    }


def median_probe_dicts(probes: list[dict]) -> dict:
    """Take element-wise median across probe wall passes while keeping deterministic counts exact."""
    if len(probes) == 1:
        return probes[0]

    # Pick median pass by run_support_geometry_total inc_wall_ns for representative reference,
    # and also compute per-metric median across passes.
    def med_int(vals: list[int]) -> int:
        return int(round(statistics.median(vals)))

    dispatch = {}
    for mod_id in sorted(probes[0]["dispatch"].keys()):
        dispatch[mod_id] = {
            k: med_int([p["dispatch"][mod_id][k] for p in probes])
            for k in probes[0]["dispatch"][mod_id]
        }

    scopes = {}
    for sc in sorted(probes[0]["scopes"].keys()):
        calls_set = {p["scopes"][sc]["calls"] for p in probes}
        if len(calls_set) != 1:
            raise RuntimeError(f"Non-deterministic scope call count for {sc}: {calls_set}")
        scopes[sc] = {
            "calls": probes[0]["scopes"][sc]["calls"],
            "inc_wall_ns": med_int([p["scopes"][sc]["inc_wall_ns"] for p in probes]),
            "exc_wall_ns": med_int([p["scopes"][sc]["exc_wall_ns"] for p in probes]),
        }

    host_ops = {}
    det_int_keys = {
        "wit_calls",
        "requests",
        "in_polys",
        "in_verts",
        "out_polys",
        "out_verts",
        "unique_keys",
        "dup_requests",
        "singleton_inputs",
    }
    for op in sorted(probes[0]["host_ops"].keys()):
        entry = {}
        for k in probes[0]["host_ops"][op]:
            vals = [p["host_ops"][op][k] for p in probes]
            if k in det_int_keys:
                if len(set(vals)) != 1:
                    raise RuntimeError(f"Non-deterministic host_op {op}.{k}: {vals}")
                entry[k] = vals[0]
            else:
                entry[k] = med_int(vals)
        host_ops[op] = entry

    # Counters & phase5_audit must be deterministic across passes
    for p in probes[1:]:
        if p["counters"] != probes[0]["counters"]:
            raise RuntimeError("Non-deterministic counters across passes")
        if p["phase5_audit"] != probes[0]["phase5_audit"]:
            raise RuntimeError("Non-deterministic phase5_audit across passes")

    return {
        "pass_count": len(probes),
        "dispatch": dispatch,
        "scopes": scopes,
        "host_ops": host_ops,
        "phase5_audit": probes[0]["phase5_audit"],
        "counters": probes[0]["counters"],
    }


def reduce_probe(ref_cells: dict[str, dict]) -> tuple[dict, dict]:
    manifest = load_json(PROBE_DIR / "manifest.json")
    rows = manifest["rows"]

    wall_cells = {}
    fuel_cells = {}
    qual_cells = {}

    for cell in SUPPORTS_ON_CELLS:
        # Wall passes
        w_rows = [r for r in rows if r["cell"] == cell and r["kind"] == "wall"]
        w_rows.sort(key=lambda r: r["pass"])
        probes = [load_json(PROBE_DIR / r["probe_file"]) for r in w_rows]
        med = median_probe_dicts(probes)
        wall_cells[cell] = {
            "passes": [
                {
                    "pass": r["pass"],
                    "exit_code": r["exit_code"],
                    "wall_s": r["wall_s"],
                    "gcode_sha256": r["gcode_sha256"],
                    "gcode_bytes": r["gcode_bytes"],
                    "reference_match": r["gcode_sha256"] == ref_cells[cell]["sha256"],
                    "run_support_geometry_total_ns": p["scopes"][
                        "run_support_geometry_total"
                    ]["inc_wall_ns"],
                    "tree_support_planner_dispatch_ns": p["dispatch"][
                        "com.core.tree-support-planner"
                    ]["total_ns"],
                    "traditional_support_planner_dispatch_ns": p["dispatch"][
                        "com.core.traditional-support-planner"
                    ]["total_ns"],
                }
                for r, p in zip(w_rows, probes)
            ],
            "wall_median_s": round(statistics.median([r["wall_s"] for r in w_rows]), 4),
            "median": med,
        }

        # Fuel pass
        f_rows = [r for r in rows if r["cell"] == cell and r["kind"] == "fuel"]
        assert len(f_rows) == 1
        fr = f_rows[0]
        fp = load_json(PROBE_DIR / fr["probe_file"])
        fuel_cells[cell] = {
            "exit_code": fr["exit_code"],
            "wall_s": fr["wall_s"],
            "gcode_sha256": fr["gcode_sha256"],
            "gcode_bytes": fr["gcode_bytes"],
            "reference_match": fr["gcode_sha256"] == ref_cells[cell]["sha256"],
            "last_fuel": fp["last_fuel"],
            "run_support_geometry_total_fuel": fp["scopes"][
                "run_support_geometry_total"
            ]["inc_fuel"],
            "scopes": {
                k: {
                    "calls": v["calls"],
                    "inc_fuel": v["inc_fuel"],
                    "exc_fuel": v["exc_fuel"],
                }
                for k, v in sorted(fp["scopes"].items())
            },
            "counters": fp["counters"],
        }

        # Qual pass
        q_rows = [r for r in rows if r["cell"] == cell and r["kind"] == "qual"]
        assert len(q_rows) == 1
        qr = q_rows[0]
        q_parsed = parse_inst_stderr(PROBE_DIR / f"{cell}.qual.p1.stderr.txt")
        qual_cells[cell] = {
            "exit_code": qr["exit_code"],
            "wall_s": qr["wall_s"],
            "gcode_sha256": qr["gcode_sha256"],
            "gcode_bytes": qr["gcode_bytes"],
            "reference_match": qr["gcode_sha256"] == ref_cells[cell]["sha256"],
            **q_parsed,
        }

    probe_wall = {
        "schema": 1,
        "captured_utc": manifest["captured_utc"],
        "reference_currency": "docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/reference.json",
        "probe_patch": "docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/probe.patch",
        "exe_sha256": manifest["exe_sha256"],
        "planner_wasm_sha256": manifest["planner_wasm_sha256"],
        "total_measured_runs": len(rows),
        "all_reference_match": all(
            r["gcode_sha256"] == ref_cells[r["cell"]]["sha256"] and r["exit_code"] == 0
            for r in rows
        ),
        "cells": wall_cells,
        "qual": qual_cells,
    }

    probe_fuel = {
        "schema": 1,
        "captured_utc": manifest["captured_utc"],
        "note": "Exact Wasmtime instruction counts under --profile with probe fuel::* scopes active. Probe scope-enter/exit overhead is reflected in run_support_geometry_total_fuel (~7.00B vs 5.85B clean on benchy, ~56.12B vs 48.56B clean on base); clean unprobed fuel deltas are in reduced/screen.json.",
        "exe_sha256": manifest["exe_sha256"],
        "planner_wasm_sha256": manifest["planner_wasm_sha256"],
        "cells": fuel_cells,
    }

    return probe_wall, probe_fuel


def reduce_screen(ref_cells: dict[str, dict]) -> dict:
    raw = load_json(SCREEN_RUNS_DIR / "screen.json")
    rows = raw["rows"]

    cells_out = {}
    for cell in SUPPORTS_ON_CELLS:
        ab_base = sorted(
            [r for r in rows if r["cell"] == cell and r["kind"] == "ab" and r["arm"] == "baseline"],
            key=lambda r: r["round"],
        )
        ab_cand = sorted(
            [r for r in rows if r["cell"] == cell and r["kind"] == "ab" and r["arm"] == "candidate"],
            key=lambda r: r["round"],
        )
        assert len(ab_base) == 3 and len(ab_cand) == 3

        pairs = []
        for b, c in zip(ab_base, ab_cand):
            assert b["round"] == c["round"]
            pairs.append(
                {
                    "round": b["round"],
                    "baseline_wall_s": b["wall_s"],
                    "candidate_wall_s": c["wall_s"],
                    "wall_saved_s": round(b["wall_s"] - c["wall_s"], 4),
                    "wall_saved_pct": round((b["wall_s"] - c["wall_s"]) / b["wall_s"] * 100.0, 2),
                    "baseline_cpu_s": b["cpu_s"],
                    "candidate_cpu_s": c["cpu_s"],
                    "cpu_saved_pct": round((b["cpu_s"] - c["cpu_s"]) / b["cpu_s"] * 100.0, 2),
                    "baseline_sha256": b["gcode_sha256"],
                    "candidate_sha256": c["gcode_sha256"],
                    "reference_match": (
                        b["gcode_sha256"] == ref_cells[cell]["sha256"]
                        and c["gcode_sha256"] == ref_cells[cell]["sha256"]
                    ),
                }
            )

        b_wall_med = round(statistics.median([r["wall_s"] for r in ab_base]), 4)
        c_wall_med = round(statistics.median([r["wall_s"] for r in ab_cand]), 4)
        b_cpu_med = round(statistics.median([r["cpu_s"] for r in ab_base]), 4)
        c_cpu_med = round(statistics.median([r["cpu_s"] for r in ab_cand]), 4)

        inst_base = parse_inst_stderr(SCREEN_RUNS_DIR / f"{cell}.baseline.inst.r1.stderr.txt")
        inst_cand = parse_inst_stderr(SCREEN_RUNS_DIR / f"{cell}.candidate.inst.r1.stderr.txt")
        prof_base = parse_profile_stderr(SCREEN_RUNS_DIR / f"{cell}.baseline.profile.r1.stderr.txt")
        prof_cand = parse_profile_stderr(SCREEN_RUNS_DIR / f"{cell}.candidate.profile.r1.stderr.txt")

        # Verify inst & profile row hashes too
        extra_rows = [
            r
            for r in rows
            if r["cell"] == cell and r["kind"] in ("inst", "profile")
        ]
        assert len(extra_rows) == 4
        extra_match = all(
            r["gcode_sha256"] == ref_cells[cell]["sha256"] and r["exit_code"] == 0
            for r in extra_rows
        )

        cells_out[cell] = {
            "reference_sha256": ref_cells[cell]["sha256"],
            "all_10_runs_reference_match": all(p["reference_match"] for p in pairs) and extra_match,
            "ab_pairs": pairs,
            "ab_summary": {
                "baseline_wall_med_s": b_wall_med,
                "candidate_wall_med_s": c_wall_med,
                "wall_saved_s": round(b_wall_med - c_wall_med, 4),
                "wall_saved_pct": round((b_wall_med - c_wall_med) / b_wall_med * 100.0, 2),
                "wall_pair_wins": sum(1 for p in pairs if p["candidate_wall_s"] < p["baseline_wall_s"]),
                "baseline_cpu_med_s": b_cpu_med,
                "candidate_cpu_med_s": c_cpu_med,
                "cpu_saved_s": round(b_cpu_med - c_cpu_med, 4),
                "cpu_saved_pct": round((b_cpu_med - c_cpu_med) / b_cpu_med * 100.0, 2),
                "cpu_pair_wins": sum(1 for p in pairs if p["candidate_cpu_s"] < p["baseline_cpu_s"]),
            },
            "instrumented": {
                "baseline": inst_base,
                "candidate": inst_cand,
            },
            "profile": {
                "baseline": prof_base,
                "candidate": prof_cand,
            },
        }

    # Supports-off inertness checks
    inert_rows = [r for r in rows if r["cell"].endswith("-off")]
    inert_out = {}
    for r in inert_rows:
        inert_out[r["cell"]] = {
            "arm": r["arm"],
            "exit_code": r["exit_code"],
            "wall_s": r["wall_s"],
            "cpu_s": r["cpu_s"],
            "gcode_sha256": r["gcode_sha256"],
            "gcode_bytes": r["gcode_bytes"],
            "reference_sha256": ref_cells[r["cell"]]["sha256"],
            "reference_match": r["gcode_sha256"] == ref_cells[r["cell"]]["sha256"],
        }

    # P5_tile falsification evidence from disk
    p5_falsification = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        p = SCREEN_DIR / f"p5test_{cell}.gcode"
        if p.exists():
            h = sha256_file(p)
            p5_falsification[cell] = {
                "gcode_sha256": h,
                "gcode_bytes": p.stat().st_size,
                "reference_sha256": ref_cells[cell]["sha256"],
                "reference_match": h == ref_cells[cell]["sha256"],
            }

    return {
        "schema": 1,
        "captured_utc": raw["captured_utc"],
        "note": "Pure-guest exact prototype screen (P1+P2+P3+P4+P5_zeroclone in modules/core-modules/tree-support-planner/src/lib.rs; zero host/WIT edits) on clean HEAD pnp_cli.exe, RAYON_NUM_THREADS=12, unsandboxed, ordinary arm. 42 measured runs (24 A/B + 2 supports-off inertness + 8 --instrument-stderr + 8 --profile) + 2 warmup runs = 44/44 byte-identical to t182/reference.json.",
        "reference_currency": "docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/reference.json",
        "exe_sha256": raw["exe_sha256"],
        "baseline_planner_wasm_sha256": raw["baseline_planner_sha256"],
        "candidate_planner_wasm_sha256": raw["candidate_planner_sha256"],
        "measured_runs_count": len(rows),
        "all_measured_runs_reference_match": all(
            r["gcode_sha256"] == ref_cells[r["cell"]]["sha256"] and r["exit_code"] == 0
            for r in rows
        ),
        "supports_on": cells_out,
        "supports_off_inertness": inert_out,
        "falsified_candidates": {
            "P5_tile_phase5_collision_tile_windowing": {
                "description": "Reusing CollisionTileIndex to window Phase-5 set-difference clip polygons by each role polygon's bbox in build_roles_batched.",
                "verdict": "REJECTED_NOT_ZERO_DRIFT",
                "cause": "Clipper2 scanbeam clamping from distant clip vertices (same mechanism documented in t175): removing distant collision vertices shifts sub-micron Phase-5 intersections on base-classic-on.",
                "cells": p5_falsification,
            }
        },
    }


def main() -> None:
    REDUCED_DIR.mkdir(parents=True, exist_ok=True)
    ref_cells = load_reference()
    probe_wall, probe_fuel = reduce_probe(ref_cells)
    screen = reduce_screen(ref_cells)

    (REDUCED_DIR / "probe_wall.json").write_text(
        json.dumps(probe_wall, indent=2) + "\n", encoding="utf-8"
    )
    (REDUCED_DIR / "probe_fuel.json").write_text(
        json.dumps(probe_fuel, indent=2) + "\n", encoding="utf-8"
    )
    (REDUCED_DIR / "screen.json").write_text(
        json.dumps(screen, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote reduced JSONs to {REDUCED_DIR}")


if __name__ == "__main__":
    main()
