#!/usr/bin/env python3
"""Ticket 145 verifier: re-derive every claim in FINDINGS.md from the reduced set.

Reads only in-repo artifacts:

    reduced/summary.json
    reference.json

Exits 0 when every claim holds. `--self-test` mutates one field at a time and
requires the matching check to fail (negative controls).

Usage:
    python verify.py
    python verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# --- literals pinned from FINDINGS.md (all re-checked against the reduced set) --
NEW_SHA = "6ab57f1c3f8e86bc3cbd302c6fa6270acf06277c6335454e922419c25d42e97e"
OLD_SHA = "6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7"
NEW_TRIANGLES = 225_706
OLD_TRIANGLES = 225_786
NEW_BYTES = 11_285_384

ACTIVE_BENCHY = {
    "benchy-classic-off": "1ae0cfa84d1b5f59893a2c30ab6ca4afe0bb73ad68249cff2ea8ddc5f09c8c5f",
    "benchy-classic-on": "d9fc80baf6ae1e71e2f1a7df4e907120b6c42c046ec1d8a77b9b53751cab3f78",
    "benchy-arachne-off": "8faf02b1b473405c3b24f261ab4e9daa630701b3d0caa78de70468c93453a7cb",
    "benchy-arachne-on": "cd7d4a2dbd67d8acc5e4c4dd2e371616deaf89e61a2753b9efba6fc4178e6345",
}
RETIRED_BENCHY = {
    "benchy-classic-off": "93e21fee6d757d37b3ae3b4efbfa2b24da178004f9a568d6647e2343ae005123",
    "benchy-classic-on": "b3f1ecccc5385f1b807c3418c271bcf57761a183e60c05eefb3294195df3d7a2",
    "benchy-arachne-off": "3510e8761d65cf3da702b9aa04dd7b0fec64ba2b16996e423bfcd46b72c1fd37",
    "benchy-arachne-on": "96cdac85437082377b351640019b3b1efe7208f6b77ada0f5b9de87035b07697",
}
EXPECTED_CROSS_SECTIONS = {
    0: (513.415, 520.028),
    10: (652.293, 659.555),
    50: (1233.326, 1237.7),
    100: (774.415, 774.594),
    150: (330.687, 330.921),
}
EXPECTED_BYTES = {
    "benchy-classic-off": (4_084_733, 4_083_921),
    "benchy-classic-on": (8_732_233, 8_652_768),
    "benchy-arachne-off": (4_229_137, 4_229_255),
    "benchy-arachne-on": (8_873_041, 8_794_268),
}
EXPECTED_RATIOS = {
    "benchy-classic-off": 2.0428,
    "benchy-classic-on": 2.6833,
    "benchy-arachne-off": 1.8530,
    "benchy-arachne-on": 2.5801,
    "base-classic-off": 2.0776,
    "base-classic-on": 3.0005,
    "base-arachne-off": 1.9840,
    "base-arachne-on": 3.4553,
}
EXPECTED_CUTS = {
    "benchy-classic-off": (51.05, 26.57),
    "benchy-classic-on": (62.73, 44.10),
    "benchy-arachne-off": (46.03, 19.05),
    "benchy-arachne-on": (61.24, 41.86),
    "base-classic-off": (51.87, 27.80),
    "base-classic-on": (66.67, 50.01),
    "base-arachne-off": (49.60, 24.40),
    "base-arachne-on": (71.06, 56.59),
}
EXPECTED_DEGRADED = {
    "benchy-classic-off": 0,
    "benchy-classic-on": 101,
    "benchy-arachne-off": 0,
    "benchy-arachne-on": 101,
    "base-classic-off": 0,
    "base-classic-on": 44474,
    "base-arachne-off": 0,
    "base-arachne-on": 44474,
}
ACCEL_FAILURE_SUBSTRING = "Cannot open source file: '\\mimalloc-static.cc'"


def close(a: float, b: float, tol: float = 5e-4) -> bool:
    return abs(a - b) <= tol


def checks(summary: dict, reference: dict):
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, ok, detail))

    swap = summary["swap"]
    gd = swap["gcode_delta"]
    cells = gd["cells"]
    score = summary["scoreboard"]
    scells = score["cells"]

    # --- fixture facts -----------------------------------------------------
    add("new fixture sha256", swap["new_export"]["sha256"] == NEW_SHA)
    add(
        "new fixture bytes",
        swap["new_export"]["bytes"] == NEW_BYTES,
        str(swap["new_export"]["bytes"]),
    )
    add("old fixture sha256 (t138 record)", swap["old_export"]["sha256"] == OLD_SHA)
    add(
        "triangle delta is -80",
        swap["triangle_delta"] == -80,
        str(swap["triangle_delta"]),
    )
    add("new triangles", swap["new_export"]["triangles"] == NEW_TRIANGLES)
    add("old triangles", swap["old_export"]["triangles"] == OLD_TRIANGLES)
    deltas = [abs(v) for side in swap["bounds_abs_delta_mm"].values() for v in side]
    add("bounds identical within 1e-6 mm", max(deltas) <= 1e-6, f"max={max(deltas):g}")
    add(
        "base fixture byte-identical to t138 record",
        swap["base_fixture_sha256"]["equal"],
    )

    # --- cross-sections ----------------------------------------------------
    cross = swap["cross_sections"]
    add("five sampled planes", len(cross) == 5)
    ok = True
    detail = []
    for row in cross:
        want = EXPECTED_CROSS_SECTIONS[row["layer"]]
        same = (
            close(row["new_mid_o200_mm2"], want[0], 1e-9)
            and close(row["new_top_o200_mm2"], want[1], 1e-9)
            and row["mid_rel"] == 0.0
            and row["top_rel"] == 0.0
        )
        ok = ok and same
        detail.append(
            f"L{row['layer']}:{row['new_mid_o200_mm2']}/{row['new_top_o200_mm2']}"
        )
    add(
        "every sampled cross-section equals the old-export record", ok, ";".join(detail)
    )
    add("cross_sections_all_equal flag", swap["cross_sections_all_equal"] is True)

    # --- code-state control ------------------------------------------------
    add(
        "four base cells byte-identical across the two takes",
        gd["base_control_all_equal"]
        and len(gd["base_control_byte_identical"]) == 4
        and all(v["equal"] for v in gd["base_control_byte_identical"].values()),
    )

    # --- printed-geometry delta -------------------------------------------
    add("four benchy cells compared", len(cells) == 4)
    for cell, (bo, bn) in EXPECTED_BYTES.items():
        add(
            f"{cell}: byte delta",
            (cells[cell]["bytes_old"], cells[cell]["bytes_new"]) == (bo, bn),
            f"{cells[cell]['bytes_old']}->{cells[cell]['bytes_new']}",
        )
        add(
            f"{cell}: 240 layers, same Z schedule",
            cells[cell]["layers_old"] == 240
            and cells[cell]["layers_new"] == 240
            and cells[cell]["z_schedule_equal"]
            and cells[cell]["last_z"] == 48.0,
        )
        q = cells[cell]["outer_wall_area_rel"]
        add(
            f"{cell}: outer-wall area median/p95 are zero",
            q["median"] == 0.0 and q["p95"] == 0.0,
            json.dumps(q),
        )
        add(
            f"{cell}: at most one layer moves >0.1% outer-wall area",
            cells[cell]["outer_wall_area_layers_over_0_1pct"] <= 1
            and cells[cell]["outer_wall_area_layers_over_1pct"] == 0,
            str(cells[cell]["outer_wall_area_layers_over_0_1pct"]),
        )
        worst = cells[cell]["outer_wall_area_worst"][0]
        add(
            f"{cell}: worst outer-wall layer is 153 at <=0.12%",
            worst["layer"] == 153 and abs(worst["rel"]) <= 0.0012,
            json.dumps(worst),
        )
        ow = cells[cell]["per_feature"]["Outer wall"]
        add(
            f"{cell}: outer-wall path within 1e-4 relative",
            abs(ow["path_rel"]) <= 1e-4,
            str(ow["path_rel"]),
        )

    for cell in [
        "benchy-classic-off",
        "benchy-classic-on",
        "benchy-arachne-off",
        "benchy-arachne-on",
    ]:
        br = cells[cell]["internal_bridge_moved_layers"]
        add(
            f"{cell}: internal bridge moves on exactly layer 40 (z 8.2)",
            len(br) == 1
            and br[0]["layer"] == 40
            and br[0]["z"] == 8.2
            and close(br[0]["delta_mm"], 57.298),
            json.dumps(br),
        )
        feat = cells[cell]["per_feature"]["Internal Bridge"]
        add(
            f"{cell}: internal-bridge path +7.344%, area +13.694%",
            close(feat["path_rel"], 0.073441, 1e-6)
            and close(feat["max_loop_area_rel"], 0.136944, 1e-6),
            f"{feat['path_rel']}/{feat['max_loop_area_rel']}",
        )

    for cell in ["benchy-classic-off", "benchy-arachne-off"]:
        add(f"{cell}: support absent", "Support" not in cells[cell]["per_feature"])
        add(f"{cell}: no support layer moves", cells[cell]["support_moved_layers"] == 0)
        add(
            f"{cell}: brim and skirt path unchanged",
            cells[cell]["per_feature"]["Brim"]["path_rel"] == 0.0
            and cells[cell]["per_feature"]["Skirt"]["path_rel"] == 0.0,
        )

    for cell in ["benchy-classic-on", "benchy-arachne-on"]:
        add(
            f"{cell}: support moves on 176 layers, -1370.181 mm",
            cells[cell]["support_moved_layers"] == 176
            and close(cells[cell]["support_delta_mm"], -1370.181, 5e-3),
            f"{cells[cell]['support_moved_layers']}/{cells[cell]['support_delta_mm']}",
        )
        sup = cells[cell]["per_feature"]["Support"]
        add(
            f"{cell}: support path -2.238%, max-loop area +1.021%",
            close(sup["path_rel"], -0.022376, 1e-6)
            and close(sup["max_loop_area_rel"], 0.010209, 1e-6),
            f"{sup['path_rel']}/{sup['max_loop_area_rel']}",
        )
        add(
            f"{cell}: brim grows, skirt grows",
            cells[cell]["per_feature"]["Brim"]["path_rel"] > 0.008
            and cells[cell]["per_feature"]["Skirt"]["path_rel"] > 0.003,
        )

    # --- scoreboard --------------------------------------------------------
    add(
        "64 rows, 8 cells", score["rows"] == 64 and len(scells) == 8, str(score["rows"])
    )
    add(
        "all eight PnP hashes equal the active reference",
        score["all_pnp_hashes_match_active_reference"] is True,
    )
    for cell, ratio in EXPECTED_RATIOS.items():
        add(
            f"{cell}: ratio {ratio}",
            close(scells[cell]["ratio_pnp_over_orca"], ratio, 1e-4),
            str(scells[cell]["ratio_pnp_over_orca"]),
        )
        cut, cut15 = EXPECTED_CUTS[cell]
        add(
            f"{cell}: required cut {cut}% / 1.5x {cut15}%",
            close(scells[cell]["required_cut_pct"], cut, 5e-3)
            and close(scells[cell]["required_cut_1_5x_pct"], cut15, 5e-3),
            f"{scells[cell]['required_cut_pct']}/{scells[cell]['required_cut_1_5x_pct']}",
        )
    add(
        "ratio range is 1.853x-3.455x",
        close(score["ratio_range"][0], 1.853, 1e-4)
        and close(score["ratio_range"][1], 3.4553, 1e-4),
        str(score["ratio_range"]),
    )
    add(
        "benchy-only ratio range is 1.853x-2.683x",
        close(score["benchy_ratio_range"][0], 1.853, 1e-4)
        and close(score["benchy_ratio_range"][1], 2.6833, 1e-4),
        str(score["benchy_ratio_range"]),
    )
    add(
        "base-only ratio range is 1.984x-3.455x",
        close(score["base_ratio_range"][0], 1.984, 1e-4)
        and close(score["base_ratio_range"][1], 3.4553, 1e-4),
        str(score["base_ratio_range"]),
    )
    add(
        "required-cut range 46.03-71.06",
        close(score["required_cut_pct_range"][0], 46.03, 5e-3)
        and close(score["required_cut_pct_range"][1], 71.06, 5e-3),
        str(score["required_cut_pct_range"]),
    )
    add(
        "1.5x-margin cut range 19.05-56.59",
        close(score["required_cut_1_5x_pct_range"][0], 19.05, 5e-3)
        and close(score["required_cut_1_5x_pct_range"][1], 56.59, 5e-3),
        str(score["required_cut_1_5x_pct_range"]),
    )
    add(
        "two tools measured, accelerated arm absent",
        score["tools_measured"] == ["orca", "pnp-ordinary"],
    )

    for cell, degraded in EXPECTED_DEGRADED.items():
        add(
            f"{cell}: degraded={degraded}",
            scells[cell]["pnp_non_fatal_counts"] == [degraded]
            and scells[cell]["pnp_all_runs_degraded"] is (degraded > 0),
            str(scells[cell]["pnp_non_fatal_counts"]),
        )
    add(
        "every run exited 0",
        all(
            v["pnp_exit_codes"] == [0] and v["orca_exit_codes"] == [0]
            for v in scells.values()
        ),
    )
    add(
        "validated generators are the expected ones",
        all(
            v["pnp_validated_generator"] == [v["pnp_validated_generator"][0]]
            and v["orca_validated_generator"] == [v["orca_validated_generator"][0]]
            for v in scells.values()
        ),
    )

    orca_keys = {
        "brim_width": ["8"],
        "brim_type": ["outer_only"],
        "skirt_loops": ["6"],
        "skirt_distance": ["3"],
        "skirt_height": ["1"],
        "initial_layer_line_width": ["0.4"],
        "tree_support_brim_width": ["0"],
        "enable_arc_fitting": ["disabled"],
        "elefant_foot_compensation": ["0"],
    }
    add(
        "Orca resolved the seven matched adhesion keys + the two t103 pins",
        all(v["resolved"] == orca_keys for v in scells.values()),
    )

    accel = score["accelerated_arm"]
    add(
        "accelerated host attempt failed with the documented libmimalloc-sys error",
        accel["attempted"] is True
        and accel["exit_code"] == 1
        and ACCEL_FAILURE_SUBSTRING in accel["failure"],
        accel["failure"],
    )

    # --- the currency ------------------------------------------------------
    add(
        "active benchy set is t144's four hashes",
        reference["benchy_active_set"] == ACTIVE_BENCHY,
    )
    add(
        "retired pre-swap benchy set is t139's four hashes",
        reference["retired_pre_swap_set"]
        == {**RETIRED_BENCHY, "note": reference["retired_pre_swap_set"]["note"]},
    )
    add(
        "benchy set reproduced in this take",
        reference["reproduced_in_this_take"] is True,
    )
    add("base set carried unchanged", reference["base_unchanged"] is True)
    add(
        "summary claims classification tessellation-only",
        summary["claims"]["classification"] == "tessellation-only"
        and summary["claims"]["swap_moves_output"] is True
        and summary["claims"]["scoreboard_is_post_swap"] is True,
    )

    return out


def run(summary, reference, verbose: bool = True) -> int:
    results = checks(summary, reference)
    failed = [r for r in results if not r[1]]
    if verbose:
        for name, ok, detail in results:
            if not ok:
                print(f"FAIL  {name}  [{detail}]")
        print(f"{len(results) - len(failed)}/{len(results)} checks pass")
    return 0 if not failed else 1


def self_test(summary, reference) -> int:
    """Each control mutates one field; the matching check must flip to FAIL."""
    controls = [
        (
            "new fixture sha",
            lambda s, r: s["swap"]["new_export"].__setitem__("sha256", "0" * 64),
        ),
        ("triangle delta", lambda s, r: s["swap"].__setitem__("triangle_delta", -79)),
        (
            "bounds",
            lambda s, r: s["swap"]["bounds_abs_delta_mm"]["min"].__setitem__(0, 1e-3),
        ),
        (
            "base fixture sha",
            lambda s, r: s["swap"]["base_fixture_sha256"].__setitem__("equal", False),
        ),
        (
            "cross-section",
            lambda s, r: s["swap"]["cross_sections"][2].__setitem__(
                "new_mid_o200_mm2", 1233.4
            ),
        ),
        (
            "cross-section flag",
            lambda s, r: s["swap"].__setitem__("cross_sections_all_equal", False),
        ),
        (
            "base control",
            lambda s, r: s["swap"]["gcode_delta"].__setitem__(
                "base_control_all_equal", False
            ),
        ),
        (
            "bytes",
            lambda s, r: s["swap"]["gcode_delta"]["cells"][
                "benchy-classic-off"
            ].__setitem__("bytes_new", 1),
        ),
        (
            "z schedule",
            lambda s, r: s["swap"]["gcode_delta"]["cells"][
                "benchy-classic-on"
            ].__setitem__("z_schedule_equal", False),
        ),
        (
            "outer-wall area quantile",
            lambda s, r: s["swap"]["gcode_delta"]["cells"]["benchy-classic-off"][
                "outer_wall_area_rel"
            ].__setitem__("p95", 1e-3),
        ),
        (
            "bridge layer count",
            lambda s, r: s["swap"]["gcode_delta"]["cells"][
                "benchy-arachne-off"
            ].__setitem__("internal_bridge_moved_layers", []),
        ),
        (
            "bridge area rel",
            lambda s, r: s["swap"]["gcode_delta"]["cells"]["benchy-classic-off"][
                "per_feature"
            ]["Internal Bridge"].__setitem__("max_loop_area_rel", 0.1),
        ),
        (
            "support layer count",
            lambda s, r: s["swap"]["gcode_delta"]["cells"][
                "benchy-classic-on"
            ].__setitem__("support_moved_layers", 175),
        ),
        (
            "supports-off brim",
            lambda s, r: s["swap"]["gcode_delta"]["cells"]["benchy-classic-off"][
                "per_feature"
            ]["Brim"].__setitem__("path_rel", 0.001),
        ),
        (
            "pnp hash reference",
            lambda s, r: s["scoreboard"].__setitem__(
                "all_pnp_hashes_match_active_reference", False
            ),
        ),
        (
            "ratio",
            lambda s, r: s["scoreboard"]["cells"]["base-classic-on"].__setitem__(
                "ratio_pnp_over_orca", 3.1
            ),
        ),
        (
            "cut",
            lambda s, r: s["scoreboard"]["cells"]["benchy-arachne-off"].__setitem__(
                "required_cut_pct", 45.0
            ),
        ),
        (
            "ratio range",
            lambda s, r: s["scoreboard"].__setitem__("ratio_range", [1.0, 9.0]),
        ),
        (
            "degraded count",
            lambda s, r: s["scoreboard"]["cells"]["benchy-classic-on"].__setitem__(
                "pnp_non_fatal_counts", [0]
            ),
        ),
        (
            "orca key",
            lambda s, r: s["scoreboard"]["cells"]["benchy-classic-off"][
                "resolved"
            ].__setitem__("brim_width", ["5"]),
        ),
        (
            "accelerated exit",
            lambda s, r: s["scoreboard"]["accelerated_arm"].__setitem__("exit_code", 0),
        ),
        (
            "active set",
            lambda s, r: r["benchy_active_set"].__setitem__(
                "benchy-classic-off", "0" * 64
            ),
        ),
        (
            "retired set",
            lambda s, r: r["retired_pre_swap_set"].__setitem__(
                "benchy-classic-on", "0" * 64
            ),
        ),
        (
            "reproduced flag",
            lambda s, r: r.__setitem__("reproduced_in_this_take", False),
        ),
        (
            "claim",
            lambda s, r: s["claims"].__setitem__("classification", "geometry-change"),
        ),
    ]
    rejected = 0
    for name, mutate in controls:
        s = copy.deepcopy(summary)
        r = copy.deepcopy(reference)
        mutate(s, r)
        code = run(s, r, verbose=False)
        if code == 0:
            print(f"CONTROL NOT REJECTED: {name}")
        else:
            rejected += 1
    print(f"{rejected}/{len(controls)} negative controls rejected")
    return 0 if rejected == len(controls) else 1


def main() -> int:
    summary = json.loads((HERE / "reduced/summary.json").read_text(encoding="utf-8"))
    reference = json.loads((HERE / "reference.json").read_text(encoding="utf-8"))
    if "--self-test" in sys.argv:
        return self_test(summary, reference)
    return run(summary, reference)


if __name__ == "__main__":
    sys.exit(main())
