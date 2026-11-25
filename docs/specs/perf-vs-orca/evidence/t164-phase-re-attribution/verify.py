#!/usr/bin/env python3
"""t164 phase re-attribution -- re-derive every quoted number from the committed set.

Inputs (all committed, stdlib only):
  reduced/board.json         this take's paired scoreboard
  reduced/instrumented.json  this take's instrumented split (three passes/cell)
  reduced/arithmetic.json    the destination arithmetic
  evidence/t151-skirt-hull-contour/reference.json  the matched job's currency

Every check is re-derived here, never trusted from the artifacts:
  C1  protocol shape (8 cells x {3 measured, 1 warmup} x 2 tools, interleaved)
  C2  per-tool medians and spreads from the run arrays
  C3  ratios and required cuts
  C4  output identity, every run against t151's active set
  C5  clean runs (exit 0, degraded=false, zero fatal/non-fatal)
  C6  Orca matched keys, from each run's resolved appendix
  C7  instrumented reconciliation and share arithmetic, per pass
  C8  instrumented output identity, every pass
  C9  the destination arithmetic and its summary
  C10 the ticket's headline claims

Usage:
  python verify.py              # 0 = every claim holds
  python verify.py --self-test  # negative controls must all be rejected
"""

from __future__ import annotations

import argparse
import copy
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REFERENCE = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
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
CANONICAL_PREPASS_S = {
    "benchy-off": 0.0505,
    "benchy-on": 0.5498,
    "base-off": 2.1030,
    "base-on": 10.4448,
}
EXPECTED_KEYS = {
    "resolved_brim_width": "8",
    "resolved_brim_type": "outer_only",
    "resolved_skirt_loops": "6",
    "resolved_skirt_distance": "3",
    "resolved_skirt_height": "1",
    "resolved_initial_layer_line_width": "0.4",
    "resolved_tree_support_brim_width": "0",
    "resolved_enable_arc_fitting": "disabled",
    "resolved_elefant_foot_compensation": "0",
}

# C10: the headline claims as written in the ticket and FINDINGS. Re-derived
# below; a prose number that drifts from the artifacts fails here.
HEADLINE = {
    # As written in the ticket and FINDINGS. Each is re-derived below; a prose
    # number that drifts from the artifacts fails C10.
    "identity_cells": "8/8",
    "cells_winning_at_canonical_prepass": 8,
    "binding_cell": "benchy-arachne-on",
    "binding_result": 0.890,
    "prepass_share_range_pct": [67.0, 87.2],
    "per_layer_share_range_pct": [9.3, 26.3],
    "required_prepass_cut_range_pct": [34.1, 84.9],
}


def close(a: float, b: float, tol: float = 5e-4) -> bool:
    return abs(a - b) <= tol


def canonical_prepass(cell: str) -> float:
    fixture, _gen, supports = cell.split("-", 2)
    return CANONICAL_PREPASS_S[f"{fixture}-{supports}"]


def fail(errors: list[str], msg: str) -> None:
    errors.append(msg)


def check(board: dict, split: dict, arith: dict, refs: dict) -> list[str]:
    errors: list[str] = []

    # C1 protocol shape
    proto = board.get("protocol", {})
    if proto.get("cells") != 8:
        fail(errors, f"C1 cells is {proto.get('cells')}, expected 8")
    if proto.get("runs_per_cell_tool") != 3:
        fail(errors, "C1 runs_per_cell_tool is not 3")
    if proto.get("warmups_per_cell_tool") != 1:
        fail(errors, "C1 warmups_per_cell_tool is not 1")
    if proto.get("measured_rows") != 48:
        fail(errors, f"C1 measured_rows is {proto.get('measured_rows')}, expected 48")
    if proto.get("warmup_rows") != 16:
        fail(errors, f"C1 warmup_rows is {proto.get('warmup_rows')}, expected 16")
    if set(board.get("cells", {})) != set(CELLS):
        fail(errors, "C1 cell set is not the matched 8")

    for cell in CELLS:
        b = board["cells"][cell]
        orca = b["orca"]
        pnp = b["pnp_ordinary"]

        # C2 medians and spreads
        if len(orca["wall_s"]) != 3 or len(pnp["wall_s"]) != 3:
            fail(errors, f"C2 {cell}: expected 3 wall samples per tool")
            continue
        if not close(statistics.median(orca["wall_s"]), orca["wall_median_s"]):
            fail(errors, f"C2 {cell}: Orca wall median does not match its runs")
        if not close(statistics.median(pnp["wall_s"]), pnp["wall_median_s"]):
            fail(errors, f"C2 {cell}: PnP wall median does not match its runs")
        if not close(statistics.median(orca["cpu_s"]), orca["cpu_median_s"]):
            fail(errors, f"C2 {cell}: Orca CPU median does not match its runs")
        if not close(statistics.median(pnp["cpu_s"]), pnp["cpu_median_s"]):
            fail(errors, f"C2 {cell}: PnP CPU median does not match its runs")
        for tool, row in (("orca", orca), ("pnp", pnp)):
            med = statistics.median(row["wall_s"])
            spread = (
                100.0 * (max(row["wall_s"]) - min(row["wall_s"])) / med if med else 0.0
            )
            if not close(spread, row["wall_spread_pct"], 5e-3):
                fail(errors, f"C2 {cell}: {tool} wall spread does not match its runs")

        # C3 ratio and required cut
        ratio = pnp["wall_median_s"] / orca["wall_median_s"]
        if not close(ratio, b["ratio_ordinary_over_orca"], 5e-5):
            fail(
                errors,
                f"C3 {cell}: ratio {ratio:.5f} != recorded {b['ratio_ordinary_over_orca']}",
            )
        cut = 100.0 * (1.0 - 1.0 / ratio)
        if not close(cut, b["required_cut_ordinary_pct"], 5e-3):
            fail(
                errors,
                f"C3 {cell}: required cut {cut:.3f} != recorded {b['required_cut_ordinary_pct']}",
            )

        # C4 output identity
        if not b["identity"]:
            fail(errors, f"C4 {cell}: identity flag is false")
        if b["hash_reference"] != refs[cell]:
            fail(errors, f"C4 {cell}: recorded reference hash is not t151's")
        for h in b["hash_runs"]:
            if h != refs[cell]:
                fail(errors, f"C4 {cell}: a PnP run hash is not t151's ({h[:12]})")

        # C5 clean runs
        if b["exit_codes"] != ["0"]:
            fail(errors, f"C5 {cell}: exit codes are {b['exit_codes']}")
        if b["degraded_pnp"] != ["false"]:
            fail(errors, f"C5 {cell}: PnP degraded flags are {b['degraded_pnp']}")
        if b["fatal_error_count_pnp"] != ["0"]:
            fail(
                errors, f"C5 {cell}: PnP fatal counts are {b['fatal_error_count_pnp']}"
            )
        if b["non_fatal_error_count_pnp"] != ["0"]:
            fail(
                errors,
                f"C5 {cell}: PnP non-fatal counts are {b['non_fatal_error_count_pnp']}",
            )
        if b["completion_status_pnp"] != ["ok"]:
            fail(
                errors,
                f"C5 {cell}: PnP completion status is {b['completion_status_pnp']}",
            )

        # C6 matched keys
        for key, expected in EXPECTED_KEYS.items():
            if b["matched_keys"].get(key) != [expected]:
                fail(
                    errors,
                    f"C6 {cell}: {key} is {b['matched_keys'].get(key)}, expected ['{expected}']",
                )
        if not b["matched_keys_ok"]:
            fail(errors, f"C6 {cell}: matched_keys_ok is false")

        # C7 instrumented reconciliation and shares
        s = split["cells"][cell]
        if s["repeat_count"] != 3:
            fail(errors, f"C7 {cell}: repeat_count is {s['repeat_count']}, expected 3")
        if len(s["passes"]) != 3:
            fail(errors, f"C7 {cell}: expected 3 instrumented passes")
        pass_shares = []
        for i, p in enumerate(s["passes"]):
            total = p["slice_complete_ms"]
            inner = sum(p["phases_ms"].values())
            if inner + p["outside_phases_ms"] != total:
                fail(
                    errors, f"C7 {cell} pass{i + 1}: phases + outside != slice_complete"
                )
            shares = {k: v / total for k, v in p["phases_ms"].items()}
            shares["outside"] = p["outside_phases_ms"] / total
            if not close(sum(shares.values()), 1.0, 2e-6):
                fail(errors, f"C7 {cell} pass{i + 1}: shares do not sum to 1")
            pass_shares.append(shares)
            if p.get("degraded") is not False:
                fail(errors, f"C7 {cell} pass{i + 1}: degraded is not false")
            if p.get("fatal_error_count") != 0 or p.get("non_fatal_error_count") != 0:
                fail(errors, f"C7 {cell} pass{i + 1}: error counts are not zero")
            if p["layer_count"] != s["passes"][0]["layer_count"]:
                fail(
                    errors, f"C7 {cell} pass{i + 1}: layer count differs between passes"
                )
        # recorded per-pass shares
        for i, recorded in enumerate(s["pass_shares"]):
            for k, v in recorded.items():
                if not close(v, pass_shares[i][k], 2e-6):
                    fail(
                        errors,
                        f"C7 {cell} pass{i + 1}: recorded share {k} does not match the pass",
                    )
        # median block
        names = sorted(pass_shares[0])
        for name in names:
            med = statistics.median(r[name] for r in pass_shares)
            if not close(med, s["median"]["shares"][name], 2e-6):
                fail(
                    errors, f"C7 {cell}: median share {name} does not match the passes"
                )
        if not close(
            s["median"]["slice_complete_ms"],
            statistics.median(p["slice_complete_ms"] for p in s["passes"]),
            1.0,
        ):
            fail(errors, f"C7 {cell}: median slice_complete does not match the passes")
        # repeat spread
        spread_pp = 0.0
        for name in names:
            values = [r[name] for r in pass_shares]
            spread_pp = max(spread_pp, 100.0 * (max(values) - min(values)))
        if not close(spread_pp, s["repeat_max_share_delta_pp"], 0.02):
            fail(errors, f"C7 {cell}: repeat share spread does not match the passes")

        # C8 instrumented output identity
        for i, p in enumerate(s["passes"]):
            if p["gcode_sha256"] != refs[cell]:
                fail(errors, f"C8 {cell} pass{i + 1}: instrumented gcode is not t151's")

        # C9 arithmetic
        r = arith["cells"][cell]
        if not close(r["orca_s"], orca["wall_median_s"], 5e-4):
            fail(errors, f"C9 {cell}: arithmetic Orca wall does not match the board")
        if not close(r["pnp_s"], pnp["wall_median_s"], 5e-4):
            fail(errors, f"C9 {cell}: arithmetic PnP wall does not match the board")
        if not close(r["ratio_of_record"], b["ratio_ordinary_over_orca"], 5e-5):
            fail(errors, f"C9 {cell}: arithmetic ratio does not match the board")
        if not close(r["required_cut"], b["required_cut_ordinary_pct"] / 100.0, 5e-6):
            fail(errors, f"C9 {cell}: arithmetic required cut does not match the board")
        for name, share in r["phase_shares"].items():
            if not close(share, s["median"]["shares"][name], 6e-5):
                fail(
                    errors,
                    f"C9 {cell}: arithmetic share {name} does not match the split",
                )
        prepass_s = r["phase_shares"]["prepass"] * r["pnp_s"]
        if not close(prepass_s, r["prepass_s"], 5e-4):
            fail(errors, f"C9 {cell}: prepass_s is not share x wall")
        other_s = r["pnp_s"] - prepass_s
        if not close(other_s, r["other_than_prepass_s"], 5e-4):
            fail(errors, f"C9 {cell}: other_than_prepass_s is not the remainder")
        if not close(r["canonical_prepass_s"], canonical_prepass(cell), 1e-9):
            fail(errors, f"C9 {cell}: canonical prepass is not t110's value")
        result = (r["canonical_prepass_s"] + other_s) / r["orca_s"]
        if not close(result, r["prepass_at_canonical_result"], 5e-4):
            fail(errors, f"C9 {cell}: prepass_at_canonical_result does not re-derive")
        budget = r["orca_s"] - other_s
        if not close(budget, r["prepass_budget_s"], 5e-4):
            fail(errors, f"C9 {cell}: prepass_budget_s does not re-derive")
        need = max(0.0, (r["canonical_prepass_s"] + other_s) - r["orca_s"])
        if not close(need, r["needed_from_other_than_prepass_s"], 5e-4):
            fail(errors, f"C9 {cell}: needed_from_other does not re-derive")

    # C9 summary
    winners = [
        c for c in CELLS if arith["cells"][c]["prepass_at_canonical_result"] < 1.0
    ]
    s = arith["summary"]
    if s["cells_winning_at_canonical_prepass"] != len(winners):
        fail(errors, "C9 summary winner count does not re-derive")
    if sorted(s["winning_cells"]) != sorted(winners):
        fail(errors, "C9 summary winning-cell list does not re-derive")
    if sorted(s["losing_cells"]) != sorted(set(CELLS) - set(winners)):
        fail(errors, "C9 summary losing-cell list does not re-derive")
    binding = max(CELLS, key=lambda c: arith["cells"][c]["prepass_at_canonical_result"])
    if s["binding_cell"] != binding:
        fail(errors, f"C9 binding cell is {s['binding_cell']}, re-derived {binding}")

    # C10 headline claims
    ident = sum(1 for c in CELLS if board["cells"][c]["identity"])
    if HEADLINE["identity_cells"] == "8/8" and ident != 8:
        fail(errors, f"C10 identity is {ident}/8, headline claims 8/8")
    if HEADLINE["binding_cell"] != binding:
        fail(errors, "C10 headline binding cell does not match the re-derivation")
    if not close(HEADLINE["binding_result"], arith["cells"][binding]["prepass_at_canonical_result"], 5e-4):
        fail(errors, "C10 headline binding result does not match the re-derivation")
    if HEADLINE["cells_winning_at_canonical_prepass"] != len(winners):
        fail(errors, "C10 headline winner count does not match the re-derivation")
    ranges = {
        "prepass_share_range_pct": [
            100.0 * min(arith["cells"][c]["phase_shares"]["prepass"] for c in CELLS),
            100.0 * max(arith["cells"][c]["phase_shares"]["prepass"] for c in CELLS),
        ],
        "per_layer_share_range_pct": [
            100.0 * min(arith["cells"][c]["phase_shares"]["per_layer"] for c in CELLS),
            100.0 * max(arith["cells"][c]["phase_shares"]["per_layer"] for c in CELLS),
        ],
        "required_prepass_cut_range_pct": [
            100.0 * min(arith["cells"][c]["required_prepass_cut"] for c in CELLS),
            100.0 * max(arith["cells"][c]["required_prepass_cut"] for c in CELLS),
        ],
    }
    for key, derived in ranges.items():
        if not close(HEADLINE[key][0], derived[0], 0.05) or not close(
            HEADLINE[key][1], derived[1], 0.05
        ):
            fail(errors, f"C10 headline {key} does not match the re-derivation")

    return errors


def load(reduced_dir: Path | None = None) -> tuple[dict, dict, dict, dict]:
    base = reduced_dir or (HERE / "reduced")
    board = json.loads((base / "board.json").read_text(encoding="utf-8"))
    split = json.loads((base / "instrumented.json").read_text(encoding="utf-8"))
    arith = json.loads((base / "arithmetic.json").read_text(encoding="utf-8"))
    refs = json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]
    return board, split, arith, refs


def mutations() -> list[tuple[str, str, callable]]:
    """(name, which artifact, mutator) pairs for the negative controls."""

    def set_path(obj, path, value):
        cur = obj
        for key in path[:-1]:
            cur = cur[key]
        cur[path[-1]] = value

    def bump(obj, path, delta):
        cur = obj
        for key in path[:-1]:
            cur = cur[key]
        cur[path[-1]] = cur[path[-1]] + delta

    return [
        (
            "board: perturb a wall median",
            "board",
            lambda d: bump(
                d, ["cells", "base-classic-on", "pnp_ordinary", "wall_median_s"], 0.011
            ),
        ),
        (
            "board: flip a ratio",
            "board",
            lambda d: set_path(
                d, ["cells", "benchy-arachne-off", "ratio_ordinary_over_orca"], 0.5
            ),
        ),
        (
            "board: change a run hash",
            "board",
            lambda d: set_path(
                d, ["cells", "base-classic-off", "hash_runs", 0], "0" * 64
            ),
        ),
        (
            "board: mark a run degraded",
            "board",
            lambda d: set_path(
                d, ["cells", "base-arachne-on", "degraded_pnp"], ["true"]
            ),
        ),
        (
            "board: break a matched key",
            "board",
            lambda d: set_path(
                d,
                ["cells", "benchy-classic-on", "matched_keys", "resolved_brim_width"],
                ["5"],
            ),
        ),
        (
            "board: break a wall spread",
            "board",
            lambda d: bump(
                d, ["cells", "benchy-classic-off", "orca", "wall_spread_pct"], 3.0
            ),
        ),
        (
            "split: perturb a median share",
            "split",
            lambda d: bump(
                d, ["cells", "base-classic-on", "median", "shares", "prepass"], 0.01
            ),
        ),
        (
            "split: break reconciliation",
            "split",
            lambda d: bump(
                d, ["cells", "base-classic-off", "passes", 0, "outside_phases_ms"], 7
            ),
        ),
        (
            "split: change an instrumented hash",
            "split",
            lambda d: set_path(
                d, ["cells", "benchy-arachne-on", "passes", 1, "gcode_sha256"], "1" * 64
            ),
        ),
        (
            "split: break the per-pass share",
            "split",
            lambda d: set_path(
                d, ["cells", "benchy-classic-off", "pass_shares", 0, "prepass"], 0.5
            ),
        ),
        (
            "split: break the repeat spread",
            "split",
            lambda d: set_path(
                d, ["cells", "base-arachne-off", "repeat_max_share_delta_pp"], 9.99
            ),
        ),
        (
            "arith: change a canonical prepass",
            "arith",
            lambda d: set_path(
                d, ["cells", "base-classic-on", "canonical_prepass_s"], 1.0
            ),
        ),
        (
            "arith: break needed_from_other",
            "arith",
            lambda d: bump(
                d,
                ["cells", "benchy-classic-on", "needed_from_other_than_prepass_s"],
                0.01,
            ),
        ),
        (
            "arith: break the winner count",
            "arith",
            lambda d: bump(d, ["summary", "cells_winning_at_canonical_prepass"], 1),
        ),
        (
            "arith: move the binding cell",
            "arith",
            lambda d: set_path(d, ["summary", "binding_cell"], "benchy-classic-off"),
        ),
        (
            "arith: break the prepass budget",
            "arith",
            lambda d: bump(d, ["cells", "base-arachne-off", "prepass_budget_s"], 0.02),
        ),
    ]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--reduced", default="", help="alternate reduced/ directory")
    a = ap.parse_args()

    board, split, arith, refs = load(Path(a.reduced) if a.reduced else None)
    errors = check(board, split, arith, refs)
    if errors:
        print(f"FAIL: {len(errors)} claim(s) do not re-derive")
        for e in errors:
            print(f"  - {e}")
        return 1
    print("PASS: every claim re-derives from the committed set")

    if a.self_test:
        rejected = 0
        for name, which, mutate in mutations():
            b, s, r = copy.deepcopy(board), copy.deepcopy(split), copy.deepcopy(arith)
            target = {"board": b, "split": s, "arith": r}[which]
            mutate(target)
            found = check(b, s, r, refs)
            if found:
                rejected += 1
                print(f"  control rejected: {name}")
            else:
                print(f"  CONTROL NOT REJECTED: {name}")
        print(f"self-test: {rejected}/{len(mutations())} mutations rejected")
        if rejected != len(mutations()):
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
