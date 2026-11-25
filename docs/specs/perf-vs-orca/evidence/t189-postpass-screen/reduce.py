#!/usr/bin/env python3
"""Reduce raw t189 probe + screen captures under target/t189/ into committed
reduced JSONs under docs/specs/perf-vs-orca/evidence/t189-postpass-screen/reduced/.
"""
from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PROBE_RAW = REPO / "target" / "t189" / "probe" / "runs" / "probe_raw.json"
PROBE_DIR = REPO / "target" / "t189" / "probe" / "runs"
SCREEN_RAW = REPO / "target" / "t189" / "screen" / "screen_raw.json"
SCREEN_DIR = REPO / "target" / "t189" / "screen"
T182_REF = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
REDUCED = HERE / "reduced"

ALL_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]

ABLATION_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]

ABLATION_CONFIGS = [
    "baseline",
    "pp2_only",
    "pp3_only",
    "pp2_pp3_host_only",
    "pp1_only",
    "full_pp1_pp2_pp3",
]

FIN_MODULES = [
    "com.core.overhang-classifier-default",
    "com.core.part-cooling",
    "com.core.skirt-brim",
    "com.core.wipe-tower",
]

POSTPASS_WASM_MODULES = FIN_MODULES + ["com.core.machine-gcode-emit"]


def parse_qual_and_profile(stderr_path: Path) -> tuple[dict, dict | None]:
    """Parse qualification counters and optional profile_summary from a stderr log."""
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    profile_data = None
    with stderr_path.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"code":1200' in line or '"code": 1200' in line or "exact-Z occupancy" in line:
                code_1200 += 1
            s = line.strip()
            if not s.startswith("{"):
                continue
            if '"slice_complete"' not in s and '"profile_summary"' not in s:
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
            elif ev == "profile_summary":
                prof = obj.get("profile", {})
                mods = {}
                for row in prof.get("modules", []) or []:
                    mid = row.get("module_id") or row.get("module")
                    if mid in POSTPASS_WASM_MODULES:
                        mods[mid] = {
                            "calls": row.get("calls"),
                            "total_fuel": row.get("total_fuel"),
                            "self_fuel": row.get("self_fuel"),
                            "total_wall_ns": row.get("total_wall_ns"),
                        }
                profile_data = {
                    "fuel_total": prof.get("fuel_total"),
                    "wall_total_ns": prof.get("wall_total_ns"),
                    "postpass_modules": mods,
                }
    qual = {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
    }
    return qual, profile_data


def structure_probes(probe_list: list[dict]) -> dict:
    """Group raw T189_PROBE event list into a deterministic keyed structure."""
    out: dict = {
        "fin_dispatch": {},
        "fin_apply": {},
    }
    for rec in probe_list:
        kind = rec["kind"]
        payload = {k: v for k, v in rec.items() if k != "kind"}
        if kind in ("fin_dispatch", "fin_apply"):
            mod = payload.pop("module")
            out[kind][mod] = payload
        else:
            out[kind] = payload
    return out


def median_dict(dicts: list[dict]) -> dict:
    """Take median of numeric values across dicts; assert non-numeric values match."""
    res = {}
    for k in dicts[0].keys():
        vals = [d[k] for d in dicts]
        if isinstance(vals[0], bool):
            assert all(v == vals[0] for v in vals), f"bool mismatch for {k}: {vals}"
            res[k] = vals[0]
        elif isinstance(vals[0], int):
            res[k] = int(statistics.median(vals))
        elif isinstance(vals[0], float):
            res[k] = round(float(statistics.median(vals)), 5)
        elif isinstance(vals[0], dict):
            res[k] = median_dict(vals)
        else:
            assert all(v == vals[0] for v in vals), f"value mismatch for {k}: {vals}"
            res[k] = vals[0]
    return res


def reduce_probe(ref: dict) -> dict:
    raw = json.loads(PROBE_RAW.read_text(encoding="utf-8"))
    refs = ref["active_set"]
    ref_bytes = ref["bytes"]

    cells_out = {}
    total_runs = 0
    for cell in ALL_CELLS:
        cdata = raw["cells"][cell]
        passes_out = []
        for p in cdata["passes"]:
            total_runs += 1
            stderr_p = PROBE_DIR / f"{cell}_p{p['pass']}.stderr.jsonl"
            qual, _ = parse_qual_and_profile(stderr_p)
            structured = structure_probes(p["probes"])
            ref_match = (p["sha256"] == refs[cell]) and (p["bytes"] == ref_bytes[cell])
            assert ref_match, f"probe hash/bytes mismatch on {cell} p{p['pass']}"
            passes_out.append({
                "pass": p["pass"],
                "wall_s": round(p["wall_s"], 5),
                "cpu_s": round(p["cpu_s"], 5),
                "gcode_sha256": p["sha256"],
                "gcode_bytes": p["bytes"],
                "reference_match": ref_match,
                "slice_complete_ms": p["slice_complete_ms"],
                "phase_ms": p["phase_ms"],
                "postpass_stage_ms": p["postpass_stage_ms"],
                "postpass_module_ms": p["postpass_module_ms"],
                "qual": qual,
                "probes": structured,
            })

        # Compute 3-pass medians
        med_probes = {
            "fin_dispatch": {
                m: median_dict([p["probes"]["fin_dispatch"][m] for p in passes_out])
                for m in FIN_MODULES
            },
            "fin_apply": {
                m: median_dict([p["probes"]["fin_apply"][m] for p in passes_out])
                for m in FIN_MODULES
            },
        }
        for key in (
            "postpass_reconcile",
            "gcode_emit",
            "gcode_postprocess",
            "gcode_serialize_inner",
            "gcode_serialize_wrapper",
            "run_tail",
        ):
            med_probes[key] = median_dict([p["probes"][key] for p in passes_out])

        med = {
            "wall_s": round(float(statistics.median(p["wall_s"] for p in passes_out)), 5),
            "cpu_s": round(float(statistics.median(p["cpu_s"] for p in passes_out)), 5),
            "slice_complete_ms": int(
                statistics.median(p["slice_complete_ms"] for p in passes_out)
            ),
            "phase_ms": median_dict([p["phase_ms"] for p in passes_out]),
            "postpass_stage_ms": median_dict([p["postpass_stage_ms"] for p in passes_out]),
            "postpass_module_ms": median_dict([p["postpass_module_ms"] for p in passes_out]),
            "probes": med_probes,
        }

        # Profile pass (parsed directly from stderr log for full fuel accuracy)
        total_runs += 1
        pp = cdata["profile_pass"]
        prof_stderr = PROBE_DIR / f"{cell}_profile.stderr.jsonl"
        prof_qual, prof_data = parse_qual_and_profile(prof_stderr)
        assert prof_data is not None, f"missing profile_summary in {prof_stderr}"
        prof_ref_match = (pp["sha256"] == refs[cell]) and (pp["bytes"] == ref_bytes[cell])
        assert prof_ref_match, f"probe profile hash/bytes mismatch on {cell}"

        cells_out[cell] = {
            "passes": passes_out,
            "median": med,
            "profile_pass": {
                "wall_s": round(pp["wall_s"], 5),
                "cpu_s": round(pp["cpu_s"], 5),
                "gcode_sha256": pp["sha256"],
                "gcode_bytes": pp["bytes"],
                "reference_match": prof_ref_match,
                "qual": prof_qual,
                "fuel_total": prof_data["fuel_total"],
                "postpass_modules": prof_data["postpass_modules"],
            },
        }

    return {
        "schema": "t189_probe_wall_v1",
        "captured_utc": raw["timestamp_utc"],
        "head_commit": "07b7ecf6",
        "rayon_num_threads": raw["rayon_num_threads"],
        "exe_sha256": raw["exe_sha256"],
        "total_runs": total_runs,
        "cells": cells_out,
    }


def reduce_screen(ref: dict) -> dict:
    raw = json.loads(SCREEN_RAW.read_text(encoding="utf-8"))
    refs = ref["active_set"]
    ref_bytes = ref["bytes"]

    ab_out = {}
    measured_runs = 0
    for cell in ALL_CELLS:
        pairs_out = []
        for p in raw["ab"][cell]["pairs"]:
            measured_runs += 2
            b = p["baseline"]
            c = p["candidate"]
            b_ok = (b["sha256"] == refs[cell]) and (b["bytes"] == ref_bytes[cell])
            c_ok = (c["sha256"] == refs[cell]) and (c["bytes"] == ref_bytes[cell])
            assert b_ok and c_ok, f"ab hash/bytes mismatch on {cell} r{p['round']}"
            pairs_out.append({
                "round": p["round"],
                "order": "base_then_cand" if p["round"] % 2 == 1 else "cand_then_base",
                "baseline_wall_s": b["wall_s"],
                "candidate_wall_s": c["wall_s"],
                "baseline_cpu_s": b["cpu_s"],
                "candidate_cpu_s": c["cpu_s"],
                "baseline_sha256": b["sha256"],
                "candidate_sha256": c["sha256"],
                "baseline_bytes": b["bytes"],
                "candidate_bytes": c["bytes"],
                "reference_match": b_ok and c_ok,
            })
        ab_out[cell] = {"pairs": pairs_out}

    inst_out = {}
    for cell in ALL_CELLS:
        inst_out[cell] = {}
        for arm in ("baseline", "candidate"):
            measured_runs += 1
            rec = raw["instrumented"][cell][arm]
            ok = (rec["sha256"] == refs[cell]) and (rec["bytes"] == ref_bytes[cell])
            assert ok, f"inst hash/bytes mismatch on {cell} {arm}"
            inst_out[cell][arm] = {
                "wall_s": rec["wall_s"],
                "cpu_s": rec["cpu_s"],
                "gcode_sha256": rec["sha256"],
                "gcode_bytes": rec["bytes"],
                "reference_match": ok,
                "inst": rec["inst"],
            }

    abl_out = {}
    for cell in ABLATION_CELLS:
        abl_out[cell] = {}
        for cfg in ABLATION_CONFIGS:
            measured_runs += 1
            rec = raw["ablation"][cell][cfg]
            ok = (rec["sha256"] == refs[cell]) and (rec["bytes"] == ref_bytes[cell])
            assert ok, f"ablation hash/bytes mismatch on {cell} {cfg}"
            abl_out[cell][cfg] = {
                "wall_s": rec["wall_s"],
                "cpu_s": rec["cpu_s"],
                "gcode_sha256": rec["sha256"],
                "gcode_bytes": rec["bytes"],
                "reference_match": ok,
                "inst": rec["inst"],
            }

    prof_out = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        prof_out[cell] = {}
        for arm, suffix in (("baseline", "base"), ("candidate", "cand")):
            measured_runs += 1
            rec = raw["profile_fuel"][cell][arm]
            ok = (rec["sha256"] == refs[cell]) and (rec["bytes"] == ref_bytes[cell])
            assert ok, f"profile hash/bytes mismatch on {cell} {arm}"
            stderr_p = SCREEN_DIR / f"prof_{cell}_{suffix}.stderr.log"
            qual, prof_data = parse_qual_and_profile(stderr_p)
            assert prof_data is not None, f"missing profile_summary in {stderr_p}"
            prof_out[cell][arm] = {
                "wall_s": rec["wall_s"],
                "cpu_s": rec["cpu_s"],
                "gcode_sha256": rec["sha256"],
                "gcode_bytes": rec["bytes"],
                "reference_match": ok,
                "inst": rec["inst"],
                "qual": qual,
                "fuel_total": prof_data["fuel_total"],
                "postpass_modules": prof_data["postpass_modules"],
            }

    return {
        "schema": "t189_screen_v1",
        "captured_utc": raw["timestamp_utc"],
        "head_commit": "07b7ecf6",
        "rayon_num_threads": raw["rayon_num_threads"],
        "baseline_exe_sha256": raw["baseline_exe_sha256"],
        "candidate_exe_sha256": raw["candidate_exe_sha256"],
        "warmup_runs": 2,
        "measured_runs": measured_runs,
        "total_runs_verified_reference_match": measured_runs + 2,
        "screened_mechanisms": [
            {
                "id": "PP-1",
                "scope": "host + 4 PostPass::LayerFinalization core guests (zero WIT changes)",
                "summary": (
                    "Zero-clone host LayerCollectionViewData::from_ir_shared (std::mem::take -> Arc -> "
                    "Arc::try_unwrap restore), lazy OnceCell LayerCollectionView::ordered_entities in "
                    "slicer-sdk/slicer-macros, skirt-brim .take(needed_layers), and part-cooling "
                    "hoisted cooling_decision_for_event."
                ),
            },
            {
                "id": "PP-2",
                "scope": "host PostPass::GCodeSerialize (crates/slicer-gcode/src/serialize.rs)",
                "summary": (
                    "Zero-alloc exact decimal formatter (push_fixed_trimmed / push_fixed_e5 with "
                    "midpoint guard and core::fmt fallback) + Rayon chunk-parallel serialize_gcode "
                    "using backward-scanned seed_e_accumulator."
                ),
            },
            {
                "id": "PP-3",
                "scope": "host PostPass::GCodeEmit + estimator + run.rs tail",
                "summary": (
                    "Parallel drop(layer_irs) via Rayon, parallel zero-alloc simplify_and_prune_indices, "
                    "pre-allocated commands + indexed annotations, single-pass parallelized estimator, "
                    "PrintEstimate reuse across GCodePostProcess gated by parallel kinematic invariance "
                    "check, and parallel ;LAYER_CHANGE scan in run.rs."
                ),
            },
        ],
        "deferred_wit_leads": [
            {
                "id": "PP-4",
                "stage": "PostPass::GCodePostProcess",
                "module": "com.core.machine-gcode-emit",
                "reason": (
                    "WIT run(commands: list<gcode-command>) passes 102K-977K commands by value and "
                    "re-emits them via 102K-977K host push calls solely to prepend start G-code and "
                    "append end G-code (+2 cmds). Fixing at the root requires an additive WIT "
                    "resource-backed gcode-stream-view + push-input-range(start, end)."
                ),
            },
            {
                "id": "PP-1b",
                "stage": "PostPass::LayerFinalization",
                "module": "com.core.part-cooling",
                "reason": (
                    "part-cooling inspects only path.role == BridgeInfill (zero point coordinates "
                    "read), yet current WIT layer-collection-view exposes only ordered-entities() "
                    "with full polyline point arrays. Fixing requires an additive WIT query such "
                    "as layer-has-role / entity-roles."
                ),
            },
        ],
        "ab": ab_out,
        "instrumented": inst_out,
        "ablation": abl_out,
        "profile_fuel": prof_out,
    }


def main() -> int:
    REDUCED.mkdir(parents=True, exist_ok=True)
    ref = json.loads(T182_REF.read_text(encoding="utf-8"))
    probe_wall = reduce_probe(ref)
    screen = reduce_screen(ref)

    (REDUCED / "probe_wall.json").write_text(
        json.dumps(probe_wall, indent=2) + "\n", encoding="utf-8"
    )
    (REDUCED / "screen.json").write_text(
        json.dumps(screen, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"Wrote {REDUCED / 'probe_wall.json'} ({probe_wall['total_runs']} runs) and "
        f"{REDUCED / 'screen.json'} ({screen['total_runs_verified_reference_match']} runs)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
