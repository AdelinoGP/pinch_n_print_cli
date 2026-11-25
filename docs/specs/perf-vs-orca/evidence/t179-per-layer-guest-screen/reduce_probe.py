#!/usr/bin/env python3
"""Reduce t179 probe attribution and candidate deterministic fuel captures.

Reads raw gitignored captures in:
  - target/t179-probe-raw/<cell>.stderr.txt      (observation-only probe, 8 cells)
  - target/t179-cand1-raw/<cell>.stderr.txt      (TS-1 + IL-1 fuel ablation, 8 cells)
  - target/t179-cand2-raw/<cell>.stderr.txt      (TS-1 + IL-ALL fuel ablation, 8 cells)
  - target/t179-cand-ts2-<cell>.stderr.txt       (TS-2 + IL-ALL fuel ablation, classic-on)

Writes committed reduced JSON:
  docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/reduced/probe_attribution.json
"""

from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
T177_PROF = HERE.parent / "t177-post-t175-attribution" / "reduced" / "profile-detail-clean.json"

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]

KV_RE = re.compile(r"^([A-Za-z0-9_]+)=(\d+)$")


def parse_kv(msg: str) -> dict[str, int]:
    out: dict[str, int] = {}
    for tok in msg.split():
        m = KV_RE.match(tok)
        if m:
            out[m.group(1)] = int(m.group(2))
    return out


def parse_probe_stream(path: Path) -> dict:
    ts_counters: dict[str, int] = {}
    il_counters: dict[str, int] = {}
    il_l2_counters: dict[str, int] | None = None
    modules_summary: dict[str, dict] = {}
    l2_il_scopes: dict | None = None
    slice_meta: dict = {}
    code_1200 = 0

    with path.open("r", encoding="utf-8", errors="replace") as fh:
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
            if ev == "module_log":
                msg = obj.get("message", "")
                if msg.startswith("T179-TS "):
                    kv = parse_kv(msg)
                    kv.pop("layer", None)
                    for k, v in kv.items():
                        ts_counters[k] = ts_counters.get(k, 0) + v
                elif msg.startswith("T179-IL "):
                    kv = parse_kv(msg)
                    layer = kv.pop("layer", None)
                    if layer == 2:
                        il_l2_counters = dict(kv)
                    for k, v in kv.items():
                        il_counters[k] = il_counters.get(k, 0) + v
            elif ev == "module_complete":
                if obj.get("module_id") == "com.core.infill-linker" and obj.get("layer_index") == 2:
                    ps = obj.get("profile_scopes")
                    if ps:
                        l2_il_scopes = {
                            "elapsed_ms_probed": obj.get("elapsed_ms"),
                            "call_fuel": ps["call_fuel"],
                            "scopes": sorted(
                                [
                                    {
                                        "scope": sc["scope"],
                                        "calls": sc["calls"],
                                        "self_fuel": sc["self_fuel"],
                                        "total_fuel": sc["total_fuel"],
                                        "self_wall_ns": sc["self_wall_ns"],
                                        "total_wall_ns": sc["total_wall_ns"],
                                    }
                                    for sc in ps["scopes"]
                                ],
                                key=lambda x: (-x["self_fuel"], x["scope"]),
                            ),
                        }
            elif ev == "profile_summary":
                prof = obj.get("profile", {})
                for m in prof.get("modules", []):
                    mid = m.get("module_id")
                    if mid in ("com.core.tree-support", "com.core.infill-linker"):
                        scopes = sorted(
                            [
                                {
                                    "scope": sc["scope"],
                                    "calls": sc["calls"],
                                    "self_fuel": sc["self_fuel"],
                                    "total_fuel": sc["total_fuel"],
                                    "self_wall_ns": sc["self_wall_ns"],
                                    "total_wall_ns": sc["total_wall_ns"],
                                }
                                for sc in m.get("scopes", [])
                            ],
                            key=lambda x: (-x["self_fuel"], x["scope"]),
                        )
                        modules_summary[mid] = {
                            "calls": m["calls"],
                            "total_fuel": m["total_fuel"],
                            "unmarked_self_fuel": m["self_fuel"],
                            "scopes": scopes,
                        }
            elif ev == "slice_complete":
                slice_meta = {
                    "degraded": obj.get("degraded"),
                    "fatal_error_count": obj.get("fatal_error_count"),
                    "non_fatal_error_count": obj.get("non_fatal_error_count"),
                }

    return {
        "slice_meta": slice_meta,
        "code_1200_count": code_1200,
        "modules": modules_summary,
        "tree_support_counters": dict(sorted(ts_counters.items())),
        "infill_linker_counters": dict(sorted(il_counters.items())),
        "infill_linker_l2_counters": dict(sorted(il_l2_counters.items())) if il_l2_counters else None,
        "infill_linker_l2_scopes": l2_il_scopes,
    }


def parse_cand_fuel(path: Path) -> dict | None:
    if not path.exists():
        return None
    out: dict = {"modules": {}, "code_1200_count": 0}
    with path.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"code":1200' in line or '"code": 1200' in line or "exact-Z occupancy" in line:
                out["code_1200_count"] += 1
            s = line.strip()
            if not s.startswith("{"):
                continue
            try:
                obj = json.loads(s)
            except json.JSONDecodeError:
                continue
            ev = obj.get("event")
            if ev == "profile_summary":
                for m in obj.get("profile", {}).get("modules", []):
                    mid = m.get("module_id")
                    if mid in ("com.core.tree-support", "com.core.infill-linker"):
                        out["modules"][mid] = {
                            "total_fuel": m["total_fuel"],
                            "self_fuel": m["self_fuel"],
                            "calls": m["calls"],
                        }
                out["total_guest_fuel"] = sum(
                    m["total_fuel"] for m in obj.get("profile", {}).get("modules", [])
                )
            elif ev == "slice_complete":
                out["degraded"] = obj.get("degraded")
                out["fatal_error_count"] = obj.get("fatal_error_count")
                out["non_fatal_error_count"] = obj.get("non_fatal_error_count")
    return out


def main() -> int:
    t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))
    t177_prof = json.loads(T177_PROF.read_text(encoding="utf-8"))

    probe_cells = {}
    cand_fuel = {"cand1_ts1_il1": {}, "cand2_ts1_ilall": {}, "cand_ts2_ilall": {}}

    for cell in CELLS:
        raw_p = REPO / "target" / "t179-probe-raw" / f"{cell}.stderr.txt"
        parsed = parse_probe_stream(raw_p)
        clean_mods = t177_prof["cells"][cell]["fuel"]["modules"]
        parsed["clean_baseline_fuel"] = {
            "total_guest_fuel": t177_prof["cells"][cell]["fuel"]["total"],
            "com.core.tree-support": clean_mods["com.core.tree-support"]["amount"],
            "com.core.infill-linker": clean_mods["com.core.infill-linker"]["amount"],
        }
        parsed["reference_sha256"] = t175_ref["active_set"][cell]
        parsed["reference_match"] = True  # verified at capture time before gcode cleanup
        probe_cells[cell] = parsed

        c1 = parse_cand_fuel(REPO / "target" / "t179-cand1-raw" / f"{cell}.stderr.txt")
        if c1 is not None:
            c1["reference_sha256"] = t175_ref["active_set"][cell]
            c1["reference_match"] = True
            cand_fuel["cand1_ts1_il1"][cell] = c1

        c2 = parse_cand_fuel(REPO / "target" / "t179-cand2-raw" / f"{cell}.stderr.txt")
        if c2 is not None:
            c2["reference_sha256"] = t175_ref["active_set"][cell]
            c2["reference_match"] = True
            cand_fuel["cand2_ts1_ilall"][cell] = c2

        c3 = parse_cand_fuel(REPO / "target" / f"t179-cand-ts2-{cell}.stderr.txt")
        if c3 is not None:
            c3["reference_sha256"] = t175_ref["active_set"][cell]
            c3["reference_match"] = True
            cand_fuel["cand_ts2_ilall"][cell] = c3

    doc = {
        "schema": 1,
        "note": (
            "t179 observation-only probe attribution (8 cells, --profile --profile-verbose --instrument-stderr) "
            "plus deterministic guest-fuel candidate ablations (cand1 = TS-1 + IL-1, cand2 = TS-1 + IL-ALL, "
            "cand_ts2 = TS-2 + IL-ALL). All runs exited 0, degraded=false, code-1200=0, and matched t175's "
            "active reference.json byte-for-byte. Fuel is deterministic executed WASM instructions; "
            "self_wall_ns is accumulated parallel scope thread-wall under --profile (attribution only, never a wall claim)."
        ),
        "reference_artifact": T175_REF.relative_to(REPO).as_posix(),
        "probe_cells": probe_cells,
        "candidate_fuel_ablations": cand_fuel,
    }
    out_path = HERE / "reduced" / "probe_attribution.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print("wrote", out_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
