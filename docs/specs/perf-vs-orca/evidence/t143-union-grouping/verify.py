#!/usr/bin/env python3
"""t143 verifier: every claim in FINDINGS.md re-derived from reduced/summary.json.

`--self-test` mutates the summary one claim at a time and requires each mutation
to be rejected, so the check set is falsifiable rather than decorative.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

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
SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]
FLOOR_PCT = 0.80


def pct(new: float, old: float) -> float:
    return (new - old) / old * 100.0


class Checks:
    def __init__(self, summary: dict):
        self.s = summary
        self.failures: list[str] = []
        self.count = 0

    def check(self, name: str, condition: bool, detail: str = "") -> None:
        self.count += 1
        if not condition:
            self.failures.append(f"{name}: {detail}")

    def close(
        self, name: str, actual: float, expected: float, tol: float, unit: str = ""
    ) -> None:
        self.check(
            name,
            abs(actual - expected) <= tol,
            f"actual {actual:.4f}{unit} vs expected {expected:.4f}{unit} (tol {tol:.4f})",
        )

    def at_least(self, name: str, actual: float, bound: float, unit: str = "") -> None:
        self.check(
            name,
            actual >= bound,
            f"actual {actual:.4f}{unit} < bound {bound:.4f}{unit}",
        )

    def at_most(self, name: str, actual: float, bound: float, unit: str = "") -> None:
        self.check(
            name,
            actual <= bound,
            f"actual {actual:.4f}{unit} > bound {bound:.4f}{unit}",
        )


def run_checks(summary: dict) -> Checks:
    c = Checks(summary)
    depth = summary.get("depth", {})
    deltas = summary.get("depth_deltas", {})
    output = summary.get("output", {})
    ab = summary.get("ab", {})
    null = summary.get("null", {})

    # ---- Output: the active reference set holds in all eight cells, both arms.
    c.check(
        "output covers 8 cells", set(output) == set(CELLS), f"cells={sorted(output)}"
    )
    for cell in CELLS:
        entry = output.get(cell, {})
        for arm in ("baseline-ordinary", "candidate-ordinary"):
            row = entry.get(arm)
            c.check(f"{cell}/{arm} recorded", row is not None, "missing row")
            if not row:
                continue
            c.check(
                f"{cell}/{arm} exit 0",
                row["exit_code"] == 0,
                f"exit={row['exit_code']}",
            )
        base = entry.get("baseline-ordinary")
        cand = entry.get("candidate-ordinary")
        if not (base and cand):
            continue
        c.check(
            f"{cell} candidate bytes == baseline bytes",
            base["gcode_bytes"] == cand["gcode_bytes"],
            f"{base['gcode_bytes']} vs {cand['gcode_bytes']}",
        )
        c.check(
            f"{cell} candidate sha == baseline sha",
            entry.get("candidate_matches_baseline") is True,
            "arms differ",
        )
        if entry.get("reference_kind") == "t139":
            c.check(
                f"{cell} baseline matches t139",
                base["matches_t139"] is True,
                f"sha={base['sha256'][:12]}",
            )
            c.check(
                f"{cell} candidate matches t139",
                cand["matches_t139"] is True,
                f"sha={cand['sha256'][:12]}",
            )
        else:
            # Replacement benchy fixture: candidate == baseline is the exactness
            # claim; the recorded baseline pins which fixture it was measured on.
            c.check(
                f"{cell} reference kind disclosed",
                entry.get("reference_kind") == "replacement-fixture-baseline",
                f"kind={entry.get('reference_kind')}",
            )
            c.check(
                f"{cell} baseline equals the recorded fixture baseline",
                entry.get("recorded_baseline") == base["sha256"],
                f"recorded={str(entry.get('recorded_baseline'))[:12]} "
                f"baseline={base['sha256'][:12]}",
            )

    # ---- Depth table: three buffering depths on the two supports-on cells.
    for cell in ("base-classic-on", "benchy-classic-on"):
        c.check(
            f"depth has {cell}",
            any(cell in cells for cells in depth.values()),
            "missing",
        )
        for depth_key in ("4", "8", "12", "16"):
            row = depth.get(depth_key, {}).get(cell)
            c.check(f"{cell} d{depth_key} recorded", row is not None, "missing")
            if not row:
                continue
            c.check(
                f"{cell} d{depth_key} reference match",
                row["reference_match"],
                f"sha={row['sha256'][:12]}",
            )
            c.check(
                f"{cell} d{depth_key} exit 0",
                row["exit_code"] == 0,
                f"exit={row['exit_code']}",
            )
            c.check(
                f"{cell} d{depth_key} union requests > 0",
                row["union_requests"] > 0,
                f"requests={row['union_requests']}",
            )
            c.check(
                f"{cell} d{depth_key} union calls > 0",
                row["union_calls"] > 0,
                f"calls={row['union_calls']}",
            )
            c.check(
                f"{cell} d{depth_key} peak working set recorded",
                row["peak_working_set_bytes"] > 0,
                "no peak RSS",
            )
            c.check(
                f"{cell} d{depth_key} first per-layer event recorded",
                row["first_per_layer_ms"] is not None,
                "no per-layer event",
            )

    # ---- Depth: the union tail collapses as the chunk deepens, and memory is the check.
    # The candidate depth is 12 (measured post-table; 4/8/16 were pre-declared).
    for cell in ("base-classic-on", "benchy-classic-on"):
        d4 = depth.get("4", {}).get(cell)
        d8 = depth.get("8", {}).get(cell)
        d12 = depth.get("12", {}).get(cell)
        d16 = depth.get("16", {}).get(cell)
        if d4 and d8 and d12 and d16:
            c.at_most(
                f"{cell} union wall falls monotonically d4 > d8 > d12",
                d12["union_wall_ms"],
                d8["union_wall_ms"],
                " ms",
            )
            c.at_most(
                f"{cell} d8 beats d4", d8["union_wall_ms"], d4["union_wall_ms"], " ms"
            )
            c.at_most(
                f"{cell} union requests preserved across depths",
                abs(d12["union_requests"] - d4["union_requests"]),
                2,
                " requests",
            )
            c.at_most(
                f"{cell} clip core falls d4 -> d12",
                d12["clip_core_wall_ms"],
                d4["clip_core_wall_ms"],
                " ms",
            )
            c.at_most(
                f"{cell} clip core beats d8 at d12",
                d12["clip_core_wall_ms"],
                d8["clip_core_wall_ms"],
                " ms",
            )
            c.at_most(
                f"{cell} first per-layer event not later at d12",
                d12["first_per_layer_ms"],
                d4["first_per_layer_ms"] + 250.0,
                " ms",
            )
            # No memory cliff at the candidate depth: +10 % over depth 4 (the
            # absolute deltas are disclosed in the table; memory is not judged
            # by the destination, DEV-026).
            c.at_most(
                f"{cell} peak RSS at d12 within 10 % of d4",
                d12["peak_working_set_bytes"],
                d4["peak_working_set_bytes"] * 1.10,
                " B",
            )
        union_deltas = deltas.get(cell, {})
        d12_delta = union_deltas.get("depth_12")
        d16_delta = union_deltas.get("depth_16")
        c.check(f"{cell} depth_12 delta recorded", d12_delta is not None, "missing")
        c.check(f"{cell} depth_16 delta recorded", d16_delta is not None, "missing")
        if d12_delta and cell == "base-classic-on":
            # The ticket's probe-side bar: the candidate's union tail must reach
            # the priced band, i.e. save > 3.5 s of the 10.1 s union wall.
            c.at_least(
                f"{cell} union wall saves >= 3,500 ms at d12",
                d12_delta["union_saved_ms"],
                3500.0,
                " ms",
            )
            # t136's merge-4 floor is 3,465 ms and LPT's 4/3 bound is 4,620 ms;
            # the candidate is a merge-3, so the bound is the merge-2 one
            # (5,480.2 ms) or better.
            c.at_most(
                f"{cell} union wall at d12 <= merge-2 floor",
                d12_delta["union_wall_ms"],
                5480.2,
                " ms",
            )
        if d16_delta and cell == "base-classic-on":
            # The merge-4 depth did NOT reach its LPT bound on this box: the
            # same 482 requests cost 24 % more thread-time, refuting the
            # merge-4 projection as a wall claim.
            c.at_least(
                f"{cell} depth 16 exceeds the LPT 4/3 bound",
                d16_delta["union_wall_ms"],
                4620.0,
                " ms",
            )
            c.at_least(
                f"{cell} d16 wall is worse than d12",
                d16_delta["union_wall_ms"],
                d12["union_wall_ms"],
                " ms",
            )

    # ---- A/B: paired gate, standing four-cell form.
    for cell in CELLS:
        row = ab.get(cell)
        c.check(f"ab has {cell}", row is not None, "missing")
        if not row:
            continue
        c.at_least(f"{cell} ab has 3 pairs", row["pair_count"], 3)
        c.check(
            f"{cell} every pair has both arms",
            len(row["pairs"]) == row["pair_count"],
            "incomplete pair",
        )
        c.check(
            f"{cell} pair order alternates",
            len({p["order"] for p in row["pairs"]}) == 2
            if row["pair_count"] >= 2
            else True,
            f"orders={sorted({p['order'] for p in row['pairs']})}",
        )
    for cell in SUPPORTS_ON:
        row = ab.get(cell)
        if not row:
            continue
        c.at_least(
            f"{cell} wall median >= 0.80 %",
            row["wall_saved_pct_median"],
            FLOOR_PCT,
            " %",
        )
        c.at_least(
            f"{cell} floor multiple >= 1x",
            row["floor_multiple"],
            1.0,
            "x",
        )
        c.at_least(
            f"{cell} wall-positive pairs majority",
            row["pairs_wall_positive"],
            (row["pair_count"] + 1) // 2,
            " pairs",
        )
    for cell in SUPPORTS_OFF:
        row = ab.get(cell)
        if not row:
            continue
        # Structurally inert (`enable_support: false` returns before
        # plan_for_object), so the four-cell form's "no median regression" reads
        # against the measured same-arm null band rather than a hard zero: the
        # benchy cells are 3 s jobs where the instrument's own spread exceeds 1 %.
        null_band = null.get(cell, {}).get("wall_delta_pct_abs_max")
        if null_band is None:
            null_band = max(
                (entry.get("wall_delta_pct_abs_max", 0.0) for entry in null.values()),
                default=0.5,
            )
        c.at_most(
            f"{cell} wall median within the null band",
            abs(row["wall_saved_pct_median"]),
            max(null_band, 0.5) + 0.25,
            " %",
        )
        cpu_band = null.get(cell, {}).get("cpu_delta_pct_abs_max")
        if cpu_band is None:
            cpu_band = max(
                (entry.get("cpu_delta_pct_abs_max", 0.0) for entry in null.values()),
                default=0.5,
            )
        c.at_most(
            f"{cell} CPU median within the null band",
            abs(row["cpu_saved_pct_median"]),
            max(cpu_band, 0.5) + 0.25,
            " %",
        )

    # ---- Route arithmetic re-derived from the measured base classic-on saving.
    base = ab.get("base-classic-on")
    if base:
        cut = summary["base_classic_on_required_cut_s"]
        c.close(
            "base classic-on saved seconds re-derived",
            base["wall_saved_s_median"],
            base["baseline_wall_median_s"] - base["candidate_wall_median_s"],
            0.01,
            " s",
        )
        c.at_least(
            "base classic-on saving meets the ticket's 3,500 ms DROP bar",
            base["wall_saved_s_median"] * 1000.0,
            3500.0,
            " ms",
        )
        share = base["wall_saved_s_median"] / cut * 100.0
        c.check(
            "base classic-on share of the 92.5 s cut is positive",
            share > 0.0,
            f"share={share:.2f}%",
        )

    return c


def self_test(summary: dict) -> int:
    baseline = run_checks(summary)
    if baseline.failures:
        print("baseline summary already failing; self-test needs a passing summary:")
        for failure in baseline.failures:
            print("  " + failure)
        return 1

    mutations: list[tuple[str, object]] = []

    def mutate(name: str, fn) -> None:
        mutated = copy.deepcopy(summary)
        fn(mutated)
        mutations.append((name, mutated))

    def set_path(path: list, value) -> None:
        def apply(doc):
            node = doc
            for key in path[:-1]:
                node = node[key]
            node[path[-1]] = value

        return apply

    # One mutation per claim group.
    mutate(
        "candidate == baseline flipped",
        set_path(["output", "base-classic-on", "candidate_matches_baseline"], False),
    )
    mutate(
        "base candidate no longer t139",
        set_path(
            ["output", "base-arachne-on", "candidate-ordinary", "matches_t139"], False
        ),
    )
    mutate(
        "base baseline no longer t139",
        set_path(
            ["output", "base-arachne-off", "baseline-ordinary", "matches_t139"], False
        ),
    )
    mutate(
        "candidate byte count moved",
        set_path(
            ["output", "base-arachne-off", "candidate-ordinary", "gcode_bytes"], 1
        ),
    )
    mutate(
        "baseline exit code non-zero",
        set_path(["output", "benchy-classic-off", "baseline-ordinary", "exit_code"], 1),
    )
    mutate(
        "benchy reference kind relabelled t139",
        set_path(["output", "benchy-classic-on", "reference_kind"], "t139"),
    )
    mutate(
        "benchy recorded baseline mismatched",
        set_path(["output", "benchy-classic-on", "recorded_baseline"], "0" * 64),
    )
    mutate(
        "depth-12 union saving small",
        set_path(
            ["depth_deltas", "base-classic-on", "depth_12", "union_saved_ms"], 10.0
        ),
    )
    mutate(
        "depth-12 union wall above merge-2 floor",
        set_path(
            ["depth_deltas", "base-classic-on", "depth_12", "union_wall_ms"], 99999.0
        ),
    )
    mutate(
        "depth-12 peak RSS above the memory bound",
        set_path(["depth", "12", "base-classic-on", "peak_working_set_bytes"], 10**12),
    )
    mutate(
        "depth-12 requests changed",
        set_path(["depth", "12", "base-classic-on", "union_requests"], 1),
    )
    mutate(
        "depth-16 union wall under the LPT bound (finding refuted)",
        set_path(
            ["depth_deltas", "base-classic-on", "depth_16", "union_wall_ms"], 3000.0
        ),
    )
    mutate(
        "depth-16 reference mismatch",
        set_path(["depth", "16", "benchy-classic-on", "reference_match"], False),
    )
    mutate(
        "depth-16 peak RSS zeroed",
        set_path(["depth", "16", "benchy-classic-on", "peak_working_set_bytes"], 0),
    )
    mutate(
        "depth-12 first layer delayed",
        set_path(["depth", "12", "base-classic-on", "first_per_layer_ms"], 99999.0),
    )
    mutate(
        "supports-on wall median below floor",
        set_path(["ab", "base-arachne-on", "wall_saved_pct_median"], 0.1),
    )
    mutate(
        "supports-on floor multiple low",
        set_path(["ab", "benchy-classic-on", "floor_multiple"], 0.1),
    )
    mutate(
        "supports-on pair count low",
        set_path(["ab", "benchy-arachne-on", "pair_count"], 1),
    )
    mutate(
        "supports-on positive pairs low",
        set_path(["ab", "base-classic-on", "pairs_wall_positive"], 0),
    )
    mutate(
        "supports-off median far outside the null band",
        set_path(["ab", "base-classic-off", "wall_saved_pct_median"], -50.0),
    )
    mutate(
        "null band shrunk below the supports-off spread",
        set_path(["null", "benchy-classic-off", "wall_delta_pct_abs_max"], 0.0),
    )
    mutate(
        "base classic-on saved seconds inconsistent",
        set_path(["ab", "base-classic-on", "wall_saved_s_median"], 0.001),
    )
    mutate(
        "base classic-on below DROP bar",
        set_path(["ab", "base-classic-on", "wall_saved_s_median"], 1.0),
    )
    mutate(
        "route share non-positive",
        set_path(["ab", "base-classic-on", "wall_saved_s_median"], 0.0),
    )

    rejected = 0
    for name, mutated in mutations:
        result = run_checks(mutated)
        if result.failures:
            rejected += 1
        else:
            print(f"NOT REJECTED: {name}")
    print(f"self-test: {rejected}/{len(mutations)} negative controls rejected")
    return 0 if rejected == len(mutations) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--summary", type=Path, default=None)
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    summary_path = args.summary or (
        Path(__file__).resolve().parent / "reduced" / "summary.json"
    )
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    if args.self_test:
        return self_test(summary)
    result = run_checks(summary)
    if result.failures:
        print(f"FAIL: {len(result.failures)} of {result.count} checks failed")
        for failure in result.failures:
            print("  " + failure)
        return 1
    print(f"PASS: {result.count} checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
