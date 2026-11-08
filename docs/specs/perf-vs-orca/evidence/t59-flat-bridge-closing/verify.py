#!/usr/bin/env python3
"""t59 flat-bridge closing reduced-set verifier.

Exits 0 when the committed reduced evidence under ``reduced/`` is internally
consistent and supports the verdict's headline relations. Reads only the
in-repo reduced set (no gitignored raw tree required).

    python docs/specs/perf-vs-orca/evidence/t59-flat-bridge-closing/verify.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"

failures: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        failures.append(msg)


def approx(a: float, b: float, tol: float) -> bool:
    return abs(a - b) <= tol


def main() -> int:
    mech = json.loads((REDUCED / "mechanism.json").read_text(encoding="utf-8"))

    for fixture in ("benchy", "base"):
        s = json.loads(
            (REDUCED / f"summary-{fixture}.json").read_text(encoding="utf-8")
        )
        calls = s["closing_calls"]
        rows = s["layers"]
        cols = {name: i for i, name in enumerate(s["layers_columns"])}

        # Structure
        check(len(rows) == calls, f"{fixture}: layers rows != closing_calls")
        check(
            s["mismatch_count"] == len(s["mismatch_layers"]),
            f"{fixture}: mismatch_count != len(mismatch_layers)",
        )
        check(
            s["verdict_ok_layers"] + s["mismatch_count"] == calls,
            f"{fixture}: verdict_ok + mismatch != calls",
        )
        check(
            set(s["mismatch_layers"])
            == {r[cols["layer"]] for r in rows if r[cols["verdict_ok"]] == 0},
            f"{fixture}: mismatch_layers disagrees with per-layer verdict_ok",
        )

        # Recompute sums from per-layer rows (stored values are rounded to 0.1 ms
        # per row; tolerance scales with row count).
        tol = 0.06 * calls + 1.0
        g_sum = sum(r[cols["global_ms"]] for r in rows)
        l_sum = sum(r[cols["local_ms"]] for r in rows)
        check(
            approx(g_sum, s["global_closing_ms_sum"], tol),
            f"{fixture}: global sum mismatch ({g_sum} vs {s['global_closing_ms_sum']})",
        )
        check(
            approx(l_sum, s["local_closing_ms_sum"], tol),
            f"{fixture}: local sum mismatch ({l_sum} vs {s['local_closing_ms_sum']})",
        )
        check(
            approx(
                (s["local_closing_ms_sum"] + s["local_clip_ms_sum"])
                / s["global_closing_ms_sum"],
                s["local_total_over_global"],
                0.001,
            ),
            f"{fixture}: local_total_over_global inconsistent with sums",
        )

        # Verdict support
        check(s["far_components"] == 0, f"{fixture}: far guard has hits (expected 0)")
        check(s["min_margin"] < 0.001, f"{fixture}: no knife-edge component recorded")
        check(s["near_components"] > 0, f"{fixture}: near-component count is zero")
        check(
            0.3 < s["box_ratio_mean"] < 0.7,
            f"{fixture}: box ratio outside the measured ~0.5 class",
        )
        check(
            s["components_total"] > 0 and s["emitted_base_total"] > 0,
            f"{fixture}: vacuous component/emission totals",
        )
        check(
            s["emitted_base_total"] != s["emitted_cand_total"]
            or s["mismatch_count"] > 0,
            f"{fixture}: no emission difference but mismatch recorded",
        )

        print(
            f"{fixture}: calls={calls} mismatch={s['mismatch_count']} "
            f"global_sum_ms={s['global_closing_ms_sum']} "
            f"local_total_over_global={s['local_total_over_global']}"
        )

    # Mechanism relations
    b = mech["benchy_layer_161"]
    r = mech["base_layer_8"]
    for name, d in (("benchy_layer_161", b), ("base_layer_8", r)):
        check(d["fidelity_ok"], f"{name}: probe reconstruction fidelity not OK")
        check(
            d["pieces_tiny"] >= d["pieces_n"] - 2,
            f"{name}: support is not sliver-dominated ({d['pieces_tiny']}/{d['pieces_n']})",
        )
        check(d["slivers"] >= 60, f"{name}: sliver population unexpectedly small")
        check(
            approx(d["closed_raw_mm2"], d["closed_raw_determinism_mm2"], 0.06),
            f"{name}: raw closing is not deterministic",
        )
        check(
            abs(d["closed_raw_mm2"] - d["closed_union_mm2"])
            / max(d["closed_raw_mm2"], 1e-9)
            > 0.01,
            f"{name}: raw and normalized path lists agree (sensitivity claim fails)",
        )
        check(
            approx(d["closed_union_mm2"], d["closed_local_mm2"], 0.5),
            f"{name}: union_ex and intersection normalizations disagree",
        )
        check(
            not approx(d["closed_big_only_mm2"], d["closed_raw_mm2"], 1.0),
            f"{name}: sliver-free closing agrees with raw (artifact claim fails)",
        )
        flips = d["component_flips"]
        check(len(flips) >= 2, f"{name}: fewer than two flipped components recorded")
        crossing = [
            c for c in flips if (c["base_frac"] >= 0.1) != (c["cand_frac"] >= 0.1)
        ]
        check(
            len(crossing) >= 1,
            f"{name}: no recorded component crosses the 10% gate (verdict-flip claim fails)",
        )

    # Fixture-specific mechanism claims
    check(
        b["closed_raw_mm2"] > b["support_area_mm2"],
        "benchy layer 161: raw closing not above support area",
    )
    # base layer 8: the raw-path-list closing is SMALLER than its own input
    # point set (artifact carving), while all normalized variants match support.
    check(
        r["closed_raw_mm2"] < r["support_area_mm2"],
        "base layer 8: raw closing not below support area (carving claim fails)",
    )
    check(
        approx(r["closed_union_mm2"], r["support_area_mm2"], 0.2),
        "base layer 8: normalized closing does not match support area",
    )
    check(
        r["refilled_local_mm2"] > r["refilled_raw_mm2"],
        "base layer 8: refilled ordering unexpected",
    )

    if failures:
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        print(f"verify: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print("verify: PASS (reduced set consistent with the t59 verdict)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
