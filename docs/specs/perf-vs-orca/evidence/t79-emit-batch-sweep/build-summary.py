#!/usr/bin/env python3
"""Assemble the committed t79 take summary from the reduced artifacts.

Inputs (all in this evidence directory): `sweep-summary.json`, `confirm.json`,
`snapshots.json`, `candidate.patch`. The summary is derived, never hand-edited:
`verify.py` re-derives its claims.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CHOSEN = {"e": 4, "c": 4096}
CONTROL = {"e": 4, "c": 256}
REFERENCE_SHA = "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319"
REFERENCE_BYTES = 43_912_792


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def main() -> int:
    sweep = load("sweep-summary.json")
    confirm = load("confirm.json")
    snapshots = load("snapshots.json")

    variants = {(v["e"], v["c"]): v for v in sweep["variants"]}
    chosen = variants.get((CHOSEN["e"], CHOSEN["c"]))
    control = variants.get((CONTROL["e"], CONTROL["c"]))
    if chosen is None or control is None:
        raise SystemExit("screening summary is missing the chosen pair or the control")
    if not all(
        v["clean"] and v["sha256"] == [REFERENCE_SHA] for v in variants.values()
    ):
        raise SystemExit("a screening variant is unclean or output-divergent")

    cap_medians = {
        c: v["planner_median_ms"]
        for (e, c), v in variants.items()
        if e == CONTROL["e"] and c in (64, 256, 1024, 4096)
    }
    if min(cap_medians, key=cap_medians.get) != CHOSEN["c"]:
        raise SystemExit(f"chosen cap is not the screening argmin: {cap_medians}")
    if confirm["wall_delta_pct_paired_median"] <= 0.0:
        raise SystemExit("confirmation did not reproduce the pre-gate failure")
    if not all(pair["sha_equal"] for pair in confirm["pairs"]):
        raise SystemExit("confirmation pair is not byte-identical")

    summary = {
        "cell": sweep["cell"],
        "control": CONTROL,
        "chosen": CHOSEN,
        "reference_sha256": REFERENCE_SHA,
        "reference_bytes": REFERENCE_BYTES,
        "screening": {
            "instrumented_only": True,
            "chunk_bracket_ms": {
                str(v["e"]): v["planner_median_ms"]
                for v in sweep["variants"]
                if v["c"] == CONTROL["c"]
            },
            "cap_bracket_ms": {str(c): m for c, m in sorted(cap_medians.items())},
            "peak_working_set_mb_max": max(
                v["peak_working_set_mb_max"] for v in sweep["variants"]
            ),
            "chosen_bracket_ms": chosen["planner_median_ms"],
            "control_bracket_ms": control["planner_median_ms"],
            "bracket_delta_pct": 100.0
            * (chosen["planner_median_ms"] / control["planner_median_ms"] - 1.0),
        },
        "confirmation": {
            "wall_delta_pct_paired_median": confirm["wall_delta_pct_paired_median"],
            "cpu_delta_pct_paired_median": confirm["cpu_delta_pct_paired_median"],
            "direction": confirm["direction"],
            "pairs": confirm["pairs"],
        },
        "snapshots": {
            "pass": snapshots["pass"],
            "ordinary": snapshots["modes"]["ordinary"],
            "accelerated": snapshots["modes"]["accelerated"],
        },
        "gate": {
            "status": "not_run_by_rule",
            "reason": (
                "The pre-gate confirmation showed the candidate slower on wall in "
                "both measured pairs (+4.97 %, +7.63 %; paired median +6.30 %) on "
                "the base cell where the screening signal lived, with "
                "byte-identical output. The standing campaign was not started; "
                "measured recommendation DROP."
            ),
        },
        "verdict": (
            "DROP - no constant change adopted; EMIT_BATCH_LAYERS stays 4 and "
            "CLIP_BATCH_MAX_REQUESTS stays 256"
        ),
    }
    (HERE / "summary.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps(summary["screening"], indent=1))
    print("confirmation wall:", summary["confirmation"]["wall_delta_pct_paired_median"])
    print("verdict:", summary["verdict"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
