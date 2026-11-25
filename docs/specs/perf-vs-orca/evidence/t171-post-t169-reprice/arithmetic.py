#!/usr/bin/env python3
"""t171 -- re-derive the post-t169 phase/stage split and re-price S5 and S6.

Every input is a committed artifact from another take or from this one; nothing
is hand-typed except the citations to prose tables whose artifacts are read
here. The script writes `reduced/arithmetic.json`, and `verify.py` re-derives
every number this take quotes from that file plus the same inputs.

Inputs:
  - `reduced/instrumented.json` (this take): phase and stage brackets, three
    passes per cell, median split. Attribution only.
  - `reduced/profile-clean.json` (this take): `--profile` per-scope self-cost,
    one pass per cell. Attribution only.
  - `../t169-offset-decimation-flat-opening/reduced/ab.json`: the post-t169
    tree's paired walls of record (`candidate_median_s`) and the savings that
    produced them.
  - `../t169-offset-decimation-flat-opening/reduced/profile.json`: the pre/post
    `polygon_ops` self-wall pair that rescales every pre-t169 price.
  - `../t163-accelerated-board/reduced/summary.json`: the last full two-tool
    board, the input t169's route table composed with its own savings.
  - `../t131-per-op-decomposition/summary.json`: the S5 census (whole-layer vs
    windowed internal-bridge offsets) and the `+0.35 mm` occupancy family.

Regimes are never mixed:
  - Shares are instrumented. `--instrument-stderr` inflates wall, so an
    instrumented wall is never a wall claim.
  - The implied phase/stage wall is `share x wall of record` — a composition
    across two windows (shares here, walls in t169's campaign), labelled an
    estimate.
  - The route table is t169's own composition (t163 ratio x t169 saving). It is
    an estimate, not a ratio of record; t161 forbids carrying a required cut
    across windows as a measurement.
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent

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


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def split_section(instrumented: dict) -> dict:
    """Per-cell phase and stage split, in ms and as shares, from the median pass."""
    out = {}
    for cell in CELLS:
        rec = instrumented["cells"][cell]
        median = rec["median"]
        total = median["slice_complete_ms"]
        shares = {k: round(v, 4) for k, v in median["shares"].items()}
        # The stage brackets are sequential inside one phase, so their sum
        # reconciles with the phase bracket (checked by verify.py).
        pass0 = rec["passes"][0]
        stage_ms = {k: int(v) for k, v in pass0["prepass_stages_ms"].items()}
        stage_ms_median = {}
        for stage in stage_ms:
            values = sorted(p["prepass_stages_ms"].get(stage, 0) for p in rec["passes"])
            stage_ms_median[stage] = int(values[len(values) // 2])
        prepass_ms = median["shares"]["prepass"] * total
        per_layer_ms = median["shares"]["per_layer"] * total
        postpass_ms = median["shares"]["postpass"] * total
        out[cell] = {
            "slice_complete_ms": total,
            "phase_shares": shares,
            "prepass_ms": round(prepass_ms, 1),
            "per_layer_ms": round(per_layer_ms, 1),
            "postpass_ms": round(postpass_ms, 1),
            "prepass_stage_ms": dict(
                sorted(stage_ms_median.items(), key=lambda kv: -kv[1])
            ),
            "prepass_stage_sum_ms": sum(stage_ms_median.values()),
            "repeat_max_share_delta_pp": rec["repeat_max_share_delta_pp"],
            "gcode_sha256": median["gcode_sha256"],
        }
    return out


def route(ab: dict, t163: dict) -> dict:
    """t169's route table, re-derived: t163 ratio x (1 - t169 saving)."""
    out = {}
    for cell in CELLS:
        orca = t163["cells"][cell]["orca"]["wall_median_s"]
        pnp_old = t163["cells"][cell]["pnp-ordinary"]["wall_median_s"]
        ratio_old = pnp_old / orca
        saving = ab[cell]["wall"]["median_saving_pct"] / 100.0
        pnp_new = ab[cell]["wall"]["candidate_median_s"]
        ratio_new = ratio_old * (1.0 - saving)
        out[cell] = {
            "t163_ratio": round(ratio_old, 4),
            "t163_pnp_s": round(pnp_old, 4),
            "t163_orca_s": round(orca, 4),
            "t169_saving_pct": round(100.0 * saving, 3),
            "estimated_ratio": round(ratio_new, 4),
            "required_cut": round(1.0 - 1.0 / ratio_new, 4),
            "pnp_wall_of_record_s": round(pnp_new, 4),
            "orca_s": round(orca, 4),
        }
    return out


def rescale(profile169: dict) -> dict:
    """Measured pre/post `polygon_ops` self wall per cell (t169's own profile)."""
    out = {}
    for cell in CELLS:
        base = profile169["arms"]["baseline"][cell]["scopes"]["wall"]
        cand = profile169["arms"]["candidate"][cell]["scopes"]["wall"]
        rec = {}
        for scope in sorted(set(base) | set(cand)):
            b = base.get(scope, 0)
            c = cand.get(scope, 0)
            rec[scope] = {
                "baseline_ns": b,
                "candidate_ns": c,
                "ratio": round(c / b, 4) if b else None,
            }
        out[cell] = rec
    return out


def s5_reprice(split: dict, route_: dict, ratios: dict, t131: dict) -> dict:
    """S5: the two internal-bridge whole-layer offsets, landed by t133.

    The price is re-derived by rescaling t131's measured census with the
    measured `polygon_ops::offset` ratio for that cell. It is an ESTIMATE: the
    census ran on a pre-t169 probe build and no instrument in this take
    separates the internal-bridge offsets from the rest of
    `PrePass::ShellClassification`.
    """
    out = {}
    for cell in ("benchy-classic-off", "base-classic-off", "base-classic-on"):
        rec = t131["cells"].get(f"{cell}@s5") or t131["cells"].get(cell) or {}
        census = rec.get("s5")
        if not census:
            continue
        whole = census["sum_whole_pos_ms"] + census["sum_whole_neg_ms"]
        windowed = census["sum_win_pos_ms"] + census["sum_win_neg_ms"]
        saved = whole - windowed
        ratio = ratios[cell]["polygon_ops::offset"]["ratio"]
        # t131's wall-equivalence factor: thread-time / stage parallelism.
        # base classic-off used 8.34x (t131 FINDINGS section 3).
        parallelism = 8.34
        wall = route_[cell]["pnp_wall_of_record_s"] * 1000.0
        out[cell] = {
            "census_layers": census["layers"],
            "whole_layer_thread_ms": round(whole, 1),
            "windowed_thread_ms": round(windowed, 1),
            "landed_saving_thread_ms": round(saved, 1),
            "offset_ratio": ratio,
            "landed_saving_rescaled_thread_ms": round(saved * ratio, 1),
            "residual_windowed_thread_ms": round(windowed * ratio, 1),
            "stage_parallelism": parallelism,
            "landed_pct_of_cell": round(100.0 * saved * ratio / parallelism / wall, 2),
            "residual_pct_of_cell": round(
                100.0 * windowed * ratio / parallelism / wall, 2
            ),
        }
    return out


def s6_reprice(route_: dict, ratios: dict, t131: dict) -> dict:
    """S6: the `+0.35 mm` occupancy offsets, landed by t132.

    Same discipline as S5: rescale t131's measured family census with the
    measured `polygon_ops::offset` ratio. The family's post-memo call count is
    its distinct-payload count, which is what the memo leaves in place.
    """
    out = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        families = t131["cells"][cell].get("s6_families") or {}
        key = next(
            (k for k in families if k.startswith("0.350000") and "none" in k), None
        )
        if key is None:
            continue
        fam = families[key]
        # The family census carries no per-repeat total; the mean call cost
        # times the repeat count is the same quantity t131's prose quotes
        # (28,521.4 ms x 431/864 = 14,228.0 ms on base classic-on).
        landed = fam["total_ms"] * fam["repeat_calls"] / fam["calls"]
        residual = fam["total_ms"] - landed
        ratio = ratios[cell]["polygon_ops::offset"]["ratio"]
        # t131 measured the planner stage's effective parallelism at 2.19x; the
        # same conversion turns this family's thread sum into its wall share.
        parallelism = 2.19
        wall = route_[cell]["pnp_wall_of_record_s"] * 1000.0
        out[cell] = {
            "family": key,
            "calls": fam["calls"],
            "distinct_payloads": fam["distinct_payloads"],
            "repeat_calls": fam["repeat_calls"],
            "family_total_thread_ms": round(fam["total_ms"], 1),
            "landed_saving_thread_ms": round(landed, 1),
            "residual_distinct_thread_ms": round(residual, 1),
            "offset_ratio": ratio,
            "landed_saving_rescaled_thread_ms": round(landed * ratio, 1),
            "residual_rescaled_thread_ms": round(residual * ratio, 1),
            "stage_parallelism": parallelism,
            "landed_pct_of_cell": round(100.0 * landed * ratio / parallelism / wall, 2),
            "residual_pct_of_cell": round(
                100.0 * residual * ratio / parallelism / wall, 2
            ),
        }
    return out


def stage_budget(split: dict, route_: dict) -> dict:
    """For each cell, what zeroing one prepass stage would leave as the ratio.

    `estimated_ratio x (1 - stage share of cell wall)` is the same
    zero-cost substitution t170 used for the offset family, applied to a stage.
    The share is instrumented (a composition, not a wall claim).
    """
    out = {}
    for cell in CELLS:
        rec = split[cell]
        total = rec["slice_complete_ms"]
        ratio = route_[cell]["estimated_ratio"]
        stages = {}
        for stage, ms in rec["prepass_stage_ms"].items():
            share = ms / total
            stages[stage] = {
                "ms": ms,
                "share_of_cell": round(share, 4),
                "share_of_prepass": round(ms / rec["prepass_ms"], 4),
                "ratio_if_zero": round(ratio * (1.0 - share), 4),
                "wins_if_zero": ratio * (1.0 - share) < 1.0,
            }
        out[cell] = {
            "estimated_ratio": ratio,
            "required_cut": route_[cell]["required_cut"],
            "prepass_share": rec["phase_shares"]["prepass"],
            "per_layer_share": rec["phase_shares"]["per_layer"],
            "postpass_share": rec["phase_shares"]["postpass"],
            "stages": stages,
        }
    return out


def census(detail: dict) -> dict:
    """Per-cell host clipper thread-time and guest fuel, from the profile table.

    Scope rows are self-cost. Host rows are accumulated thread-time under
    `--profile` (inflated, summed across concurrent workers) and guest rows are
    executed instructions; the script quote shares *within* each channel and
    never converts either to wall.
    """
    out = {}
    for cell in CELLS:
        rec = detail["cells"][cell]
        host_ns: dict[str, int] = {}
        for builtin in rec["wall"]["builtins"].values():
            for scope, row in builtin["scopes"].items():
                host_ns[scope] = host_ns.get(scope, 0) + row["amount"]
        host_total = sum(host_ns.values())
        fuel_units: dict[str, int] = {}
        for module in rec["fuel"]["modules"].values():
            for scope, row in module["scopes"].items():
                fuel_units[scope] = fuel_units.get(scope, 0) + row["amount"]
        fuel_total = rec["fuel"]["total"]
        module_rows = sorted(
            (
                (mid, row["amount"], row["calls"])
                for mid, row in rec["fuel"]["modules"].items()
            ),
            key=lambda row: -row[1],
        )
        out[cell] = {
            "host_thread_ns": dict(sorted(host_ns.items(), key=lambda kv: -kv[1])),
            "host_thread_total_ns": host_total,
            "host_scope_shares": {
                k: round(v / host_total, 4) for k, v in host_ns.items()
            },
            "guest_fuel_total": fuel_total,
            "guest_fuel_scopes": dict(
                sorted(fuel_units.items(), key=lambda kv: -kv[1])
            ),
            "guest_fuel_scope_shares": {
                k: round(v / fuel_total, 4) for k, v in fuel_units.items()
            },
            "top_modules": [
                {"module": mid, "fuel": amount, "calls": calls}
                for mid, amount, calls in module_rows[:3]
            ],
            "top_module_shares": [
                {"module": mid, "share": round(amount / fuel_total, 4)}
                for mid, amount, _calls in module_rows[:3]
            ],
        }
    return out


def main() -> int:
    instrumented = load(HERE / "reduced" / "instrumented.json")
    profile_clean = load(HERE / "reduced" / "profile-clean.json")
    profile_detail = load(HERE / "reduced" / "profile-detail-clean.json")
    t169_ab = load(EVID / "t169-offset-decimation-flat-opening" / "reduced" / "ab.json")
    t169_profile = load(
        EVID / "t169-offset-decimation-flat-opening" / "reduced" / "profile.json"
    )
    t163 = load(EVID / "t163-accelerated-board" / "reduced" / "summary.json")
    t131 = load(EVID / "t131-per-op-decomposition" / "summary.json")

    split = split_section(instrumented)
    route_ = route(t169_ab, t163)
    ratios = rescale(t169_profile)

    payload = {
        "note": (
            "ESTIMATE chain, never a ratio of record. Shares are this take's "
            "instrumented median over three passes per cell; walls of record are "
            "t169's paired candidate medians; the route table is t163's board "
            "composed with t169's measured saving; every pre-t169 price is "
            "rescaled by t169's measured polygon_ops self-wall ratio. No "
            "instrumented or --profile wall is a wall claim."
        ),
        "split": split,
        "route": route_,
        "offset_ratios": ratios,
        "stage_budget": stage_budget(split, route_),
        "s5": s5_reprice(split, route_, ratios, t131),
        "s6": s6_reprice(route_, ratios, t131),
        "census": census(profile_detail),
        "profile_scopes": {
            cell: profile_clean["cells"][cell]["scopes"] for cell in CELLS
        },
    }

    out = HERE / "reduced" / "arithmetic.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")

    print("phase shares (median of three instrumented passes):")
    for cell in CELLS:
        s = split[cell]["phase_shares"]
        print(
            f"  {cell:20s} prepass {100 * s['prepass']:5.1f}%  "
            f"per-layer {100 * s['per_layer']:5.1f}%  "
            f"postpass {100 * s['postpass']:4.1f}%  "
            f"outside {100 * s['outside']:4.1f}%"
        )
    print("route (t163 x t169 saving) and required cut:")
    for cell in CELLS:
        r = route_[cell]
        print(
            f"  {cell:20s} {r['t163_ratio']:5.3f}x -> {r['estimated_ratio']:5.3f}x  "
            f"cut {100 * r['required_cut']:5.1f}%"
        )
    print("host clipper thread-time split (attribution only):")
    for cell in CELLS:
        c = payload["census"][cell]
        sh = c["host_scope_shares"]
        print(
            f"  {cell:20s} clip {100 * sh.get('polygon_ops::clip_polygons', 0):5.1f}%  "
            f"offset {100 * sh.get('polygon_ops::offset', 0):5.1f}%  "
            f"offset2_ex {100 * sh.get('polygon_ops::offset2_ex', 0):5.1f}%  "
            f"closing {100 * sh.get('polygon_ops::closing_ex', 0):5.1f}%  "
            f"total {c['host_thread_total_ns'] / 1e9:6.1f}s thread"
        )
    print("largest prepass stages and zero-stage ratio:")
    for cell in CELLS:
        b = payload["stage_budget"][cell]
        top = sorted(b["stages"].items(), key=lambda kv: -kv[1]["ms"])[:3]
        parts = " ".join(
            f"{name.replace('PrePass::', '')} {100 * v['share_of_cell']:4.1f}%"
            f"->{v['ratio_if_zero']:4.2f}x"
            for name, v in top
        )
        print(f"  {cell:20s} {parts}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
