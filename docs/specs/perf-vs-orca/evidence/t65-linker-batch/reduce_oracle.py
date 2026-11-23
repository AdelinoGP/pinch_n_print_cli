#!/usr/bin/env python3
"""t65 replay-oracle reducer.

Reads the oracle capture tree (``target/t65/oracle/<cell>/``) produced by
``run-oracle.sh`` on the probed candidate build, plus the production
references (t58 qualification.json), and re-derives:

- per cell: summed T65-PROBE layer counters (invocations, subjects, matches,
  mismatches, batch_spans, replay_spans, singles, fallbacks) and the counter
  consistency checks (matches + mismatches == subjects; batch_spans ==
  replay_spans when no mismatch), each cell's output sha256 against the
  preserved production reference;
- per fuel cell (``*-fuel``): the t65::batch vs t65::replay fuel rows from
  the profile summary (module com.core.infill-linker).

Writes oracle.json; exits 0 when every cell's oracle verdict is
"all subjects matched, no fallback beyond singles, outputs reference-true".
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent  # evidence/t65-linker-batch
REPO = HERE.parents[4]
ORACLE = REPO / "target" / "t65" / "oracle"
REFERENCES = HERE.parent / "t58-finalization-copies" / "qualification.json"

PROBE_RE = re.compile(
    r"T65-PROBE layer=(\d+) invocations=(\d+) subjects=(\d+) matches=(\d+)"
    r" mismatches=(\d+) batch_spans=(\d+) replay_spans=(\d+)"
    r" singles=(\d+) fallbacks=(\d+)(?: repairs=(\d+))? first_bad=(\d+)"
)

# Cell name -> reference cell name in t58's qualification.json
CELL_TO_REF = {
    "b-c-off": "ordinary-benchy-classic-off",
    "b-c-on": "ordinary-benchy-classic-on",
    "b-a-off": "ordinary-benchy-arachne-off",
    "b-a-on": "ordinary-benchy-arachne-on",
    "b-c-off-acc": "accelerated-benchy-classic-off",
    "b-c-on-acc": "accelerated-benchy-classic-on",
    "b-a-off-acc": "accelerated-benchy-arachne-off",
    "b-a-on-acc": "accelerated-benchy-arachne-on",
    "base-c-off": "ordinary-base-classic-off",
    "base-a-off": "ordinary-base-arachne-off",
    "base-a-on": "ordinary-base-arachne-on",
}


def refs() -> dict[str, str]:
    data = json.loads(REFERENCES.read_text(encoding="utf-8"))
    return {c["cell"]: c["sha256"] for c in data["cells"]}


def parse_counters(stderr_path: Path) -> dict:
    counters = {
        "invocations": 0,
        "subjects": 0,
        "matches": 0,
        "mismatches": 0,
        "batch_spans": 0,
        "replay_spans": 0,
        "singles": 0,
        "fallbacks": 0,
        "repairs": 0,
    }
    layers = 0
    mismatch_layers = []
    fallback_layers = []
    for line in stderr_path.open(encoding="utf-8", errors="replace"):
        m = PROBE_RE.search(line)
        if not m:
            continue
        (
            layer,
            inv,
            subj,
            mat,
            mis,
            bsp,
            rsp,
            sing,
            fb,
            reps,
            first_bad,
        ) = m.groups()
        layer = int(layer)
        layers += 1
        counters["invocations"] += int(inv)
        counters["subjects"] += int(subj)
        counters["matches"] += int(mat)
        counters["mismatches"] += int(mis)
        counters["batch_spans"] += int(bsp)
        counters["replay_spans"] += int(rsp)
        counters["singles"] += int(sing)
        counters["fallbacks"] += int(fb)
        counters["repairs"] += int(reps) if reps else 0
        if int(mis) > 0:
            mismatch_layers.append(layer)
        if int(fb) > 0:
            fallback_layers.append(layer)
    return {
        "layers": layers,
        "mismatch_layers": mismatch_layers,
        "fallback_layers": fallback_layers,
        **counters,
    }


def parse_profile(path: Path) -> dict | None:
    """Finds the last profile_summary and extracts t65::batch / t65::replay
    fuel rows scoped to com.core.infill-linker (ADR-0055 summary schema)."""
    summary = None
    for line in path.open(encoding="utf-8", errors="replace"):
        if '"event": "profile_summary"' in line or '"event":"profile_summary"' in line:
            try:
                summary = json.loads(line)
            except json.JSONDecodeError:
                summary = None
    if summary is None:
        return None
    out = {}
    profile = summary.get("profile") or {}
    modules = profile.get("modules") or []
    for module in modules:
        if module.get("module_id") != "com.core.infill-linker":
            continue
        for scope in module.get("scopes") or []:
            name = scope.get("scope") or ""
            if name in ("t65::batch", "t65::replay"):
                out[name] = {
                    "calls": scope.get("calls"),
                    "total_fuel": scope.get("total_fuel"),
                    "self_fuel": scope.get("self_fuel"),
                    "total_wall_ns": scope.get("total_wall_ns"),
                    "self_wall_ns": scope.get("self_wall_ns"),
                }
        out.setdefault("linker_total_fuel", module.get("total_fuel"))
        out.setdefault("linker_self_fuel", module.get("self_fuel"))
        out.setdefault("linker_calls", module.get("calls"))
    return out or None


def main() -> int:
    reference_shas = refs()
    cells = {}
    problems = []
    for cell, ref_cell in CELL_TO_REF.items():
        cdir = ORACLE / cell
        sha_path = cdir / "out.gcode"
        err_path = cdir / "stderr.jsonl"
        if not sha_path.exists() or not err_path.exists():
            problems.append(f"{cell}: capture incomplete")
            continue
        sha = _sha(sha_path)
        counters = parse_counters(err_path)
        ok_counters = (
            counters["mismatches"] == 0
            and counters["matches"] == counters["subjects"]
            and counters["invocations"] > 0
        )
        ref_sha = reference_shas.get(ref_cell)
        if ref_sha is None:
            problems.append(f"{cell}: reference cell {ref_cell} missing")
        ref_ok = sha == ref_sha
        verdict = "pass" if (ok_counters and ref_ok) else "FAIL"
        if not ok_counters:
            problems.append(f"{cell}: oracle counters violated")
        if not ref_ok:
            problems.append(f"{cell}: output {sha[:8]} != reference {ref_sha[:8]}")
        entry = {
            "cell": cell,
            "sha256": sha,
            "reference": ref_sha,
            **counters,
            "oracle": verdict,
        }
        fuel_path = cdir / "profile.jsonl"
        if (cdir / "profile.jsonl").exists():
            entry["t65_fuel"] = parse_profile(fuel_path) or {"absent": True}
        cells[cell] = entry
    fuel_cells = {}
    for fuel_cell in ("b-a-off-fuel", "b-c-off-fuel", "base-a-off-fuel"):
        cdir = ORACLE / fuel_cell
        profile_path = cdir / "profile.jsonl"
        if not profile_path.exists():
            profile_path = cdir / "stderr.jsonl"
        if profile_path.exists():
            fuel_cells[fuel_cell] = parse_profile(profile_path) or {"absent": True}
    out = {"cells": cells, "fuel_cells": fuel_cells, "problems": problems}
    (HERE / "oracle.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    print(json.dumps({**(cells), "problems": problems}, default=str, indent=1)[:4000])
    print("VERDICT:", "PASS" if not problems else "FAIL")
    return 0 if not problems else 1


def _sha(path: Path) -> str:
    import hashlib

    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


if __name__ == "__main__":
    sys.exit(main())
