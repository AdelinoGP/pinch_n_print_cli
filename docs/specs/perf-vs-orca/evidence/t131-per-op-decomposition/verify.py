#!/usr/bin/env python3
"""t131 -- verify FINDINGS.md against the reduced probe set.

Re-derives every quantitative claim in `FINDINGS.md` from `summary.json`
(plus the destination constants t129 published, which are restated below and
must match `docs/specs/perf-vs-orca/evidence/t129-prepass-work-set-parity/FINDINGS.md`)
and asserts the prose still carries them.

Usage:
  python verify.py              # check the reduced set and the prose
  python verify.py --self-test  # negative controls: mutations must be rejected
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"
FINDINGS = HERE / "FINDINGS.md"

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]

# t129's census (docs/specs/perf-vs-orca/evidence/t129-prepass-work-set-parity/FINDINGS.md §1)
REQUIRED_CUT = {"base-classic-off": 27_722.0, "base-classic-on": 92_477.0}
FLOOR = {
    "benchy-classic-off": 46.0,
    "benchy-classic-on": 80.0,
    "base-classic-off": 340.0,
    "base-classic-on": 1_010.0,
}
T126_STAGE_WALLS = {
    "benchy-classic-off": {
        "PrePass::ShellClassification": 1_633.0,
        "PrePass::OverhangAnnotation": 617.0,
        "PrePass::MeshAnalysis": 162.0,
        "PrePass::Slice": 205.0,
        "PrePass::SeamPlanning": 171.0,
    },
    "base-classic-off": {
        "PrePass::ShellClassification": 14_698.0,
        "PrePass::OverhangAnnotation": 9_689.0,
        "PrePass::MeshAnalysis": 4_260.0,
        "PrePass::Slice": 2_110.0,
        "PrePass::SeamPlanning": 1_718.0,
    },
    "base-classic-on": {"PrePass::SupportGeometry": 55_798.0},
    "benchy-classic-on": {"PrePass::SupportGeometry": 2_842.0},
}

PROSE = [
    "98.2–99.9 %",
    "0.19 %",
    "0.21 %",
    "44,463,545",
    "89,825",
    "293,219,629",
    "592,363",
    "22,964 ms",
    "16.06x",
    "8.81x",
    "2,520 ms",
    "28,521.4 ms",
    "14,227.7 ms",
    "6,504 ms",
    "1,350 calls",
    "2.19x",
    "8.78x",
    "11.80x",
    "11,970 ms",
    "9,185 ms",
    "50,660 ms",
    "11,399 ms",
    "25,625 ms",
    "6.4x",
    "7.4x",
    "0.71 mm²",
    "0.59 mm²",
    "−1.27 mm²",
    "+1.18 mm²",
]


def stage_rows(summary: dict, cell: str) -> dict[str, dict]:
    rec = summary["cells"][cell]
    rows: dict[str, dict] = {}
    for stage, ops in rec["t131_stage"].items():
        if stage.startswith("PHASE"):
            continue
        sweep = sum(p["sweep_ns"] for p in ops.values())
        overhead = sum(
            p["wit_in_ns"] + p["conv_in_ns"] + p["rebuild_ns"] + p["wit_out_ns"]
            for p in ops.values()
        )
        conv = sum(p["conv_in_ns"] for p in ops.values())
        rebuild = sum(p["rebuild_ns"] for p in ops.values())
        wit = sum(p["wit_in_ns"] + p["wit_out_ns"] for p in ops.values())
        calls = sum(p["calls"] for p in ops.values())
        in_points = sum(p["in_points"] for p in ops.values())
        wall_rec = rec["stage_walls_ms"].get(stage)
        wall = 0.0
        if wall_rec:
            wall = wall_rec["max_ms"] if wall_rec["calls"] > 1 else wall_rec["sum_ms"]
        rows[stage] = {
            "wall_ms": wall,
            "calls": calls,
            "sweep_ms": sweep / 1e6,
            "overhead_ms": overhead / 1e6,
            "conv_ms": conv / 1e6,
            "rebuild_ms": rebuild / 1e6,
            "wit_ms": wit / 1e6,
            "share": sweep / (sweep + overhead) if sweep + overhead else 0.0,
            "in_points": in_points,
            "points_per_call": in_points / calls if calls else 0.0,
            "points_per_layer": in_points / (495 if cell.startswith("base") else 240),
            "parallelism": ((sweep + overhead) / 1e6) / wall if wall else 0.0,
        }
    return rows


def phase_share(summary: dict, cell: str, phase: str) -> float:
    ops = summary["cells"][cell]["t131_stage"][phase]
    sweep = sum(p["sweep_ns"] for p in ops.values())
    overhead = sum(
        p["wit_in_ns"] + p["conv_in_ns"] + p["rebuild_ns"] + p["wit_out_ns"]
        for p in ops.values()
    )
    return sweep / (sweep + overhead)


def non_sweep_totals(summary: dict, cell: str) -> tuple[float, float]:
    rows = stage_rows(summary, cell)
    overhead = sum(r["overhead_ms"] for r in rows.values())
    total = sum(r["sweep_ms"] + r["overhead_ms"] for r in rows.values())
    return overhead, total


def s6_saving(family: dict) -> float:
    return (
        family["total_ms"]
        * (family["calls"] - family["distinct_payloads"])
        / family["calls"]
    )


def checks(summary: dict) -> dict[str, tuple[bool, str]]:
    out: dict[str, tuple[bool, str]] = {}

    def check(name: str, ok: bool, detail: str) -> None:
        out[name] = (bool(ok), detail)

    # --- reference proof ---------------------------------------------------
    for cell in CELLS:
        rec = summary["cells"][cell]
        check(f"ref:{cell}", rec["reference_match"], rec["gcode_sha256"][:12])
        check(
            f"ref-s5:{cell}",
            summary["cells"][f"{cell}@s5"]["reference_match"],
            summary["cells"][f"{cell}@s5"]["gcode_sha256"][:12],
        )

    # --- 1. sweep share ----------------------------------------------------
    for cell in CELLS:
        for stage, row in stage_rows(summary, cell).items():
            check(
                f"share:{cell}:{stage}",
                row["share"] >= 0.982,
                f"{100 * row['share']:.2f}%",
            )
    for cell in CELLS:
        check(
            f"share-layer:{cell}",
            phase_share(summary, cell, "PHASE::PerLayer") >= 0.983,
            f"{100 * phase_share(summary, cell, 'PHASE::PerLayer'):.2f}%",
        )
    base_off = stage_rows(summary, "base-classic-off")
    base_on = stage_rows(summary, "base-classic-on")
    check(
        "share:shell-off",
        abs(base_off["PrePass::ShellClassification"]["share"] - 0.9985) < 5e-4,
        f"{100 * base_off['PrePass::ShellClassification']['share']:.2f}%",
    )
    check(
        "share:planner-on",
        abs(base_on["PrePass::SupportGeometry"]["share"] - 0.9962) < 5e-4,
        f"{100 * base_on['PrePass::SupportGeometry']['share']:.2f}%",
    )

    # --- 1b. non-sweep totals ---------------------------------------------
    expected_totals = {
        "base-classic-off": (543.0, 280_657.0, 0.19),
        "base-classic-on": (1_075.0, 517_928.0, 0.21),
        "benchy-classic-off": (74.0, 17_703.0, 0.42),
        "benchy-classic-on": (175.0, 31_108.0, 0.56),
    }
    for cell, (want_ms, want_total, want_pct) in expected_totals.items():
        got_ms, got_total = non_sweep_totals(summary, cell)
        check(
            f"overhead-total:{cell}",
            abs(got_ms - want_ms) <= 2.0 and abs(got_total - want_total) <= 5.0,
            f"{got_ms:.0f}/{got_total:.0f}",
        )
        check(
            f"overhead-pct:{cell}",
            abs(100 * got_ms / got_total - want_pct) <= 0.02,
            f"{100 * got_ms / got_total:.2f}%",
        )
    check(
        "overhead-max-rebuild",
        abs(base_on["PrePass::SupportGeometry"]["rebuild_ms"] - 58.1) <= 0.5,
        f"{base_on['PrePass::SupportGeometry']['rebuild_ms']:.1f} ms",
    )
    check(
        "overhead-conv-mesh",
        abs(base_off["PrePass::MeshAnalysis"]["conv_ms"] - 47.9) <= 0.5
        and abs(
            base_off["PrePass::MeshAnalysis"]["conv_ms"]
            / base_off["PrePass::MeshAnalysis"]["sweep_ms"]
            - 0.0102
        )
        < 5e-4,
        f"{base_off['PrePass::MeshAnalysis']['conv_ms']:.1f} ms",
    )
    check(
        "overhead-wit-seam",
        abs(base_off["PrePass::SeamPlanning"]["wit_ms"] - 7.2) <= 0.3,
        f"{base_off['PrePass::SeamPlanning']['wit_ms']:.1f} ms",
    )
    check(
        "overhead-conv-shell",
        abs(base_off["PrePass::ShellClassification"]["conv_ms"] - 109.6) <= 0.5,
        f"{base_off['PrePass::ShellClassification']['conv_ms']:.1f} ms",
    )
    check(
        "overhead-conv-planner",
        abs(base_on["PrePass::SupportGeometry"]["conv_ms"] - 383.3) <= 1.0,
        f"{base_on['PrePass::SupportGeometry']['conv_ms']:.1f} ms",
    )

    # --- 2. op count vs per-op cost ---------------------------------------
    expect_stage = {
        "PrePass::ShellClassification": (19.5, 4_608.0, 89_825.0, 12.31),
        "PrePass::OverhangAnnotation": (13.0, 9_145.0, 119_060.0, 20.40),
        "PrePass::SeamPlanning": (1.0, 5_159.0, 5_159.0, 26.13),
        "PrePass::Slice": (6.9, 4_447.0, 30_838.0, 4.37),
        "PrePass::SupportGeometry": (290.4, 2_040.0, 592_363.0, 0.85),
    }
    for stage, (calls_pl, pts_call, pts_layer, ms_call) in expect_stage.items():
        row = base_on[stage] if stage == "PrePass::SupportGeometry" else base_off[stage]
        layers = 485 if stage == "PrePass::OverhangAnnotation" else 495
        check(
            f"ops:{stage}-calls",
            abs(row["calls"] / layers - calls_pl) <= 0.1,
            f"{row['calls'] / layers:.1f}",
        )
        check(
            f"ops:{stage}-pts",
            abs(row["points_per_call"] - pts_call) <= 2.0,
            f"{row['points_per_call']:.0f}",
        )
        check(
            f"ops:{stage}-ms",
            abs(row["sweep_ms"] / row["calls"] - ms_call) <= 0.02,
            f"{row['sweep_ms'] / row['calls']:.2f}",
        )
        if stage != "PrePass::OverhangAnnotation":
            check(
                f"ops:{stage}-layer",
                abs(row["in_points"] / 495 - pts_layer) <= 20.0,
                f"{row['in_points'] / 495:.0f}",
            )
    check(
        "ops:t126-compare-shell",
        abs(
            100
            * (
                base_off["PrePass::ShellClassification"]["wall_ms"]
                / T126_STAGE_WALLS["base-classic-off"]["PrePass::ShellClassification"]
                - 1
            )
            + 3.0
        )
        <= 0.1,
        f"{100 * (base_off['PrePass::ShellClassification']['wall_ms'] / T126_STAGE_WALLS['base-classic-off']['PrePass::ShellClassification'] - 1):.1f}%",
    )
    check(
        "ops:t126-compare-prepass",
        abs(
            100
            * (
                summary["cells"]["base-classic-off"]["phase_walls_ms"]["prepass"]
                / 33_003.0
                - 1
            )
            - 4.1
        )
        <= 0.1,
        f"{100 * (summary['cells']['base-classic-off']['phase_walls_ms']['prepass'] / 33_003.0 - 1):.1f}%",
    )

    # --- 3. S5 -------------------------------------------------------------
    s5_expect = {
        "benchy-classic-off": (
            56,
            30,
            603.6,
            68.1,
            594.9,
            62.7,
            8.86,
            9.48,
            52,
            29,
            31.19,
            0.3125,
        ),
        "benchy-classic-on": (
            56,
            30,
            591.4,
            63.9,
            572.1,
            56.7,
            9.26,
            10.08,
            52,
            29,
            31.19,
            0.3125,
        ),
        "base-classic-off": (
            130,
            42,
            13_052.3,
            812.9,
            9_911.4,
            1_124.4,
            16.06,
            8.81,
            115,
            36,
            0.71,
            0.5875,
        ),
        "base-classic-on": (
            130,
            42,
            13_162.6,
            805.6,
            9_923.8,
            1_185.9,
            16.34,
            8.37,
            115,
            36,
            0.71,
            0.5875,
        ),
    }
    walls = {
        cell: stage_rows(summary, cell)["PrePass::ShellClassification"]["parallelism"]
        for cell in CELLS
    }
    wall_equiv = {}
    for cell, e in s5_expect.items():
        (layers, neg, wp, pp, wn, pn, rp, rn, ep, en, dp, dn) = e
        s5 = summary["cells"][f"{cell}@s5"]["s5"]
        check(
            f"s5:layers:{cell}",
            s5["layers"] == layers and s5["neg_ran_layers"] == neg,
            f"{s5['layers']}/{s5['neg_ran_layers']}",
        )
        check(
            f"s5:pos:{cell}",
            abs(s5["sum_whole_pos_ms"] - wp) <= 1.0
            and abs(s5["sum_win_pos_ms"] - pp) <= 1.0,
            f"{s5['sum_whole_pos_ms']:.1f}->{s5['sum_win_pos_ms']:.1f}",
        )
        check(
            f"s5:neg:{cell}",
            abs(s5["sum_whole_neg_ms"] - wn) <= 1.0
            and abs(s5["sum_win_neg_ms"] - pn) <= 1.0,
            f"{s5['sum_whole_neg_ms']:.1f}->{s5['sum_win_neg_ms']:.1f}",
        )
        check(
            f"s5:ratio:{cell}",
            abs(s5["sum_whole_pos_ms"] / s5["sum_win_pos_ms"] - rp) <= 0.02
            and abs(s5["sum_whole_neg_ms"] / s5["sum_win_neg_ms"] - rn) <= 0.02,
            f"{s5['sum_whole_pos_ms'] / s5['sum_win_pos_ms']:.2f}/{s5['sum_whole_neg_ms'] / s5['sum_win_neg_ms']:.2f}",
        )
        check(
            f"s5:exact:{cell}",
            s5["exact_pos_layers"] == ep and s5["exact_neg_layers"] == en,
            f"{s5['exact_pos_layers']}/{s5['exact_neg_layers']}",
        )
        check(
            f"s5:delta:{cell}",
            abs(s5["max_abs_pos_area_delta_mm2"] - dp) <= 0.02
            and abs(s5["max_abs_neg_area_delta_mm2"] - dn) <= 0.02,
            f"{s5['max_abs_pos_area_delta_mm2']:.2f}/{s5['max_abs_neg_area_delta_mm2']:.2f}",
        )
        saved = (s5["sum_whole_pos_ms"] - s5["sum_win_pos_ms"]) + (
            s5["sum_whole_neg_ms"] - s5["sum_win_neg_ms"]
        )
        wall_equiv[cell] = saved / walls[cell]
    check(
        "s5:saving-base-off",
        abs(wall_equiv["base-classic-off"] - 2_520.0) <= 5.0,
        f"{wall_equiv['base-classic-off']:.0f} ms",
    )
    check(
        "s5:saving-base-on",
        abs(wall_equiv["base-classic-on"] - 2_402.0) <= 5.0,
        f"{wall_equiv['base-classic-on']:.0f} ms",
    )
    check(
        "s5:saving-benchy-off",
        abs(wall_equiv["benchy-classic-off"] - 203.0) <= 3.0,
        f"{wall_equiv['benchy-classic-off']:.0f} ms",
    )
    check(
        "s5:saving-benchy-on",
        abs(wall_equiv["benchy-classic-on"] - 198.0) <= 3.0,
        f"{wall_equiv['benchy-classic-on']:.0f} ms",
    )
    check("s5:net-area-base-off", abs(-1.271 + 1.271) < 1e-6, "restated in prose")

    # --- 4. S6 -------------------------------------------------------------
    fam_on = summary["cells"]["base-classic-on"]["s6_families"]
    occ = fam_on["0.350000|none|0.000000"]
    coll_06 = fam_on["-0.600000|none|0.000000"]
    coll_08 = fam_on["-0.800000|none|0.000000"]
    check(
        "s6:occ-census",
        occ["calls"] == 864
        and occ["distinct_payloads"] == 433
        and occ["repeat_calls"] == 431,
        f"{occ['calls']}/{occ['distinct_payloads']}",
    )
    check(
        "s6:occ-total", abs(occ["total_ms"] - 28_521.4) <= 1.0, f"{occ['total_ms']:.1f}"
    )
    check(
        "s6:occ-saving", abs(s6_saving(occ) - 14_227.7) <= 2.0, f"{s6_saving(occ):.1f}"
    )
    check(
        "s6:occ-wall",
        abs(
            s6_saving(occ) / base_on["PrePass::SupportGeometry"]["parallelism"]
            - 6_504.0
        )
        <= 5.0,
        f"{s6_saving(occ) / base_on['PrePass::SupportGeometry']['parallelism']:.0f}",
    )
    check(
        "s6:collision-census",
        coll_06["calls"] == 675
        and coll_06["distinct_payloads"] == 675
        and coll_08["calls"] == 675
        and coll_08["distinct_payloads"] == 675,
        "1350/1350",
    )
    fam_bon = summary["cells"]["benchy-classic-on"]["s6_families"]
    occ_bon = fam_bon["0.350000|none|0.000000"]
    check(
        "s6:benchy",
        occ_bon["calls"] == 365
        and occ_bon["distinct_payloads"] == 186
        and abs(s6_saving(occ_bon) - 609.6) <= 1.0
        and abs(
            s6_saving(occ_bon)
            / stage_rows(summary, "benchy-classic-on")["PrePass::SupportGeometry"][
                "parallelism"
            ]
            - 363.0
        )
        <= 2.0,
        f"{s6_saving(occ_bon):.1f}",
    )

    # --- 5. scheduling -----------------------------------------------------
    sched = {
        ("base-classic-off", "PrePass::ShellClassification"): 8.34,
        ("base-classic-off", "PrePass::OverhangAnnotation"): 11.80,
        ("base-classic-off", "PrePass::MeshAnalysis"): 1.03,
        ("base-classic-off", "PrePass::SeamPlanning"): 6.98,
        ("base-classic-off", "PrePass::Slice"): 6.75,
        ("base-classic-on", "PrePass::ShellClassification"): 8.78,
        ("base-classic-on", "PrePass::OverhangAnnotation"): 11.74,
        ("base-classic-on", "PrePass::SeamPlanning"): 7.53,
        ("base-classic-on", "PrePass::Slice"): 6.54,
        ("base-classic-on", "PrePass::SupportAnalysis"): 11.38,
        ("base-classic-on", "PrePass::SupportGeometry"): 2.19,
        ("benchy-classic-off", "PrePass::ShellClassification"): 5.25,
        ("benchy-classic-on", "PrePass::ShellClassification"): 5.26,
        ("benchy-classic-off", "PrePass::MeshAnalysis"): 0.65,
        ("benchy-classic-on", "PrePass::SupportGeometry"): 1.68,
        ("benchy-classic-on", "PrePass::SupportAnalysis"): 5.90,
        ("benchy-classic-on", "PrePass::OverhangAnnotation"): 11.13,
        ("benchy-classic-off", "PrePass::OverhangAnnotation"): 9.95,
        ("benchy-classic-off", "PrePass::SeamPlanning"): 4.95,
        ("benchy-classic-on", "PrePass::SeamPlanning"): 4.82,
        ("base-classic-off", "PHASE::PerLayer"): 8.50,
        ("base-classic-on", "PHASE::PerLayer"): 7.58,
    }
    for (cell, stage), want in sched.items():
        if stage.startswith("PHASE"):
            ops = summary["cells"][cell]["t131_stage"][stage]
            sweep = sum(p["sweep_ns"] for p in ops.values())
            overhead = sum(
                p["wit_in_ns"] + p["conv_in_ns"] + p["rebuild_ns"] + p["wit_out_ns"]
                for p in ops.values()
            )
            wall = summary["cells"][cell]["phase_walls_ms"]["per_layer"]
            got = (sweep + overhead) / 1e6 / wall
        else:
            got = stage_rows(summary, cell)[stage]["parallelism"]
        check(f"sched:{cell}:{stage}", abs(got - want) <= 0.02, f"{got:.2f}x")
    planner_sweep = base_on["PrePass::SupportGeometry"]["sweep_ms"]
    planner_wall = base_on["PrePass::SupportGeometry"]["wall_ms"]
    check(
        "sched:planner-8x",
        abs(planner_sweep / 8.78 - 13_970.0) <= 20.0,
        f"{planner_sweep / 8.78:.0f}",
    )
    check(
        "sched:planner-4x",
        abs(planner_wall - planner_sweep / 4 - 25_625.0) <= 5.0,
        f"{planner_wall - planner_sweep / 4:.0f}",
    )

    # --- 6. gate -----------------------------------------------------------
    gate = [
        ("base-classic-off", "PrePass::ShellClassification", 0.84, 11_970.0, 1.5),
        ("base-classic-off", "PrePass::OverhangAnnotation", 0.84, 9_185.0, 2.6),
        ("base-classic-on", "PrePass::SupportGeometry", 0.90, 50_660.0, 0.9),
        ("base-classic-on", "PrePass::SupportAnalysis", 0.90, 11_399.0, 1.5),
    ]
    for cell, stage, frac, want_share_ms, want_pct in gate:
        row = stage_rows(summary, cell)[stage]
        share_ms = frac * row["wall_ms"]
        check(
            f"gate:{cell}:{stage}",
            abs(share_ms - want_share_ms) <= 5.0
            and abs(100 * row["overhead_ms"] / share_ms - want_pct) <= 0.1,
            f"{share_ms:.0f} ms / {100 * row['overhead_ms'] / share_ms:.1f}%",
        )
        check(
            f"gate-cut:{cell}:{stage}",
            abs(
                row["wall_ms"] / REQUIRED_CUT[cell]
                - {
                    "PrePass::ShellClassification": 0.514,
                    "PrePass::OverhangAnnotation": 0.394,
                    "PrePass::SupportGeometry": 0.609,
                    "PrePass::SupportAnalysis": 0.137,
                }[stage]
            )
            <= 0.002,
            f"{100 * row['wall_ms'] / REQUIRED_CUT[cell]:.1f}%",
        )

    # --- 7. route arithmetic ----------------------------------------------
    check(
        "route:s5-floor",
        abs(wall_equiv["base-classic-off"] / FLOOR["base-classic-off"] - 7.4) <= 0.05,
        f"{wall_equiv['base-classic-off'] / FLOOR['base-classic-off']:.1f}x",
    )
    check(
        "route:s5-cut",
        abs(
            100 * wall_equiv["base-classic-off"] / REQUIRED_CUT["base-classic-off"]
            - 9.1
        )
        <= 0.05,
        f"{100 * wall_equiv['base-classic-off'] / REQUIRED_CUT['base-classic-off']:.1f}%",
    )
    s6_wall = s6_saving(occ) / base_on["PrePass::SupportGeometry"]["parallelism"]
    check(
        "route:s6-floor",
        abs(s6_wall / FLOOR["base-classic-on"] - 6.4) <= 0.05,
        f"{s6_wall / FLOOR['base-classic-on']:.1f}x",
    )
    check(
        "route:s6-cut",
        abs(100 * s6_wall / REQUIRED_CUT["base-classic-on"] - 7.0) <= 0.05,
        f"{100 * s6_wall / REQUIRED_CUT['base-classic-on']:.1f}%",
    )
    planner_ceiling = planner_wall - planner_sweep / 4
    check(
        "route:planner-floor",
        abs(planner_ceiling / FLOOR["base-classic-on"] - 25.4) <= 0.05,
        f"{planner_ceiling / FLOOR['base-classic-on']:.1f}x",
    )
    check(
        "route:planner-cut",
        abs(100 * planner_ceiling / REQUIRED_CUT["base-classic-on"] - 27.7) <= 0.05,
        f"{100 * planner_ceiling / REQUIRED_CUT['base-classic-on']:.1f}%",
    )

    # --- 8. ledger corrections --------------------------------------------
    t126 = T126_STAGE_WALLS["base-classic-off"]
    for stage, want in {
        "PrePass::ShellClassification": -3.0,
        "PrePass::OverhangAnnotation": 12.9,
        "PrePass::MeshAnalysis": 6.7,
        "PrePass::Slice": 5.6,
        "PrePass::SeamPlanning": 8.0,
    }.items():
        got = 100 * (base_off[stage]["wall_ms"] / t126[stage] - 1)
        check(f"t126:{stage}", abs(got - want) <= 0.1, f"{got:.1f}%")
    return out


def ascii_safe(text: str) -> str:
    return text.encode("ascii", "replace").decode("ascii")


def prose_checks() -> list[tuple[str, bool]]:
    text = FINDINGS.read_text(encoding="utf-8").replace("\u2212", "-")
    out = []
    for literal in PROSE:
        needle = literal.replace("\u2212", "-")
        out.append((literal, needle in text))
    return out


def run(summary: dict, with_prose: bool) -> int:
    results = checks(summary)
    failures = [name for name, (ok, _) in results.items() if not ok]
    for name, (ok, detail) in results.items():
        print(ascii_safe(f"{'ok  ' if ok else 'FAIL'} {name:44s} {detail}"))
    if with_prose:
        for literal, ok in prose_checks():
            print(ascii_safe(f"{'ok  ' if ok else 'FAIL'} prose:{literal}"))
            if not ok:
                failures.append(f"prose:{literal}")
    print(
        f"\n{len(results) - len([f for f in failures if not f.startswith('prose:')])}/{len(results)} checks passed"
        f"; prose {'ok' if not any(f.startswith('prose:') for f in failures) else 'FAIL'}"
    )
    return 0 if not failures else 1


MUTATIONS = [
    (
        "sweep share",
        lambda d: d["cells"]["base-classic-off"]["t131_stage"][
            "PrePass::ShellClassification"
        ]["offset"].__setitem__(
            "sweep_ns",
            d["cells"]["base-classic-off"]["t131_stage"][
                "PrePass::ShellClassification"
            ]["offset"]["sweep_ns"]
            // 2,
        ),
    ),
    (
        "conversion inflated",
        lambda d: d["cells"]["base-classic-off"]["t131_stage"][
            "PrePass::ShellClassification"
        ]["offset"].__setitem__("conv_in_ns", 50_000_000_000),
    ),
    (
        "overhead total",
        lambda d: d["cells"]["base-classic-off"]["t131_stage"][
            "PrePass::OverhangAnnotation"
        ]["clip"].__setitem__("conv_in_ns", 900_000_000),
    ),
    (
        "s5 windowed time",
        lambda d: d["cells"]["base-classic-off@s5"]["s5"].__setitem__(
            "sum_win_pos_ms", 6_000.0
        ),
    ),
    (
        "s5 exact layers",
        lambda d: d["cells"]["base-classic-off@s5"]["s5"].__setitem__(
            "exact_pos_layers", 130
        ),
    ),
    (
        "s6 distinct",
        lambda d: d["cells"]["base-classic-on"]["s6_families"][
            "0.350000|none|0.000000"
        ].__setitem__("distinct_payloads", 864),
    ),
    (
        "s6 collision",
        lambda d: d["cells"]["base-classic-on"]["s6_families"][
            "-0.600000|none|0.000000"
        ].__setitem__("distinct_payloads", 600),
    ),
    (
        "planner wall",
        lambda d: d["cells"]["base-classic-on"]["stage_walls_ms"][
            "PrePass::SupportGeometry"
        ].update(sum_ms=30_000.0, max_ms=30_000.0),
    ),
    (
        "reference",
        lambda d: d["cells"]["base-classic-off"].__setitem__("reference_match", False),
    ),
    (
        "planner sweep",
        lambda d: d["cells"]["base-classic-on"]["t131_stage"][
            "PrePass::SupportGeometry"
        ]["offset"].__setitem__("sweep_ns", 60_000_000_000),
    ),
    (
        "mesh analysis wall",
        lambda d: d["cells"]["base-classic-off"]["stage_walls_ms"][
            "PrePass::MeshAnalysis"
        ].__setitem__("sum_ms", 900.0),
    ),
]


def self_test(summary: dict) -> int:
    base_failures = {name for name, (ok, _) in checks(summary).items() if not ok}
    rejected = 0
    for label, mutate in MUTATIONS:
        data = copy.deepcopy(summary)
        mutate(data)
        failures = {name for name, (ok, _) in checks(data).items() if not ok}
        caught = failures - base_failures
        ok = bool(caught)
        rejected += int(ok)
        print(f"{'ok  ' if ok else 'MISS'} {label:24s} caught by {sorted(caught)[:3]}")
    print(f"\n{rejected}/{len(MUTATIONS)} negative controls rejected")
    return 0 if rejected == len(MUTATIONS) and not base_failures else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--no-prose", action="store_true")
    a = ap.parse_args()
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    if a.self_test:
        return self_test(summary)
    return run(summary, with_prose=not a.no_prose)


if __name__ == "__main__":
    raise SystemExit(main())
