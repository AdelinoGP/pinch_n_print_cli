#!/usr/bin/env python3
"""Ticket 138 verifier: re-derive every headline claim from `reduced/summary.json`.

Checkout-only: needs only the committed summary. Run from anywhere:

    python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/verify.py
    python docs/specs/perf-vs-orca/evidence/t138-layer0-cross-section/verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"


class CheckFailure(AssertionError):
    pass


def expect(cond, name, detail=""):
    if not cond:
        raise CheckFailure(f"{name}: {detail}")


def close(a, b, tol):
    return abs(a - b) <= tol


# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------
def check_layer_counts(d):
    lc = d["layer_counts"]
    expect(
        lc["benchy"]["pnp_layers"] == 240 and lc["benchy"]["orca_layers"] == 240,
        "layer-count/benchy",
        json.dumps(lc["benchy"]),
    )
    expect(
        lc["base"]["pnp_layers"] == 495 and lc["base"]["orca_layers"] == 496,
        "layer-count/base",
        json.dumps(lc["base"]),
    )
    expect(
        lc["base"]["pnp_last_z"] == 99.0 and lc["base"]["orca_last_z"] == 99.2,
        "layer-count/base-z",
        json.dumps(lc["base"]),
    )
    expect(
        close(lc["base"]["object_height_mm"], 99.159767, 1e-3),
        "layer-count/base-height",
        str(lc["base"]["object_height_mm"]),
    )
    expect(
        close(lc["benchy"]["object_height_mm"], 48.0, 1e-6),
        "layer-count/benchy-height",
        str(lc["benchy"]["object_height_mm"]),
    )


def check_slice_plane(d):
    samples = d["slice_plane"]["samples"]
    expect(len(samples) == 10, "slice-plane/sample-count", str(len(samples)))
    disc = {(s["fixture"], s["layer"]) for s in samples if s["discriminating"]}
    expect(
        disc == {("base", 0), ("base", 10)},
        "slice-plane/discriminating-set",
        str(sorted(disc)),
    )
    for s in samples:
        tag = f"{s['fixture']}-L{s['layer']}"
        # The expected convention must beat the wrong plane on every sample.
        expect(
            s["pnp_vs_top"] < s["pnp_vs_mid"],
            f"slice-plane/pnp-{tag}",
            f"top={s['pnp_vs_top']} mid={s['pnp_vs_mid']}",
        )
        expect(
            s["orca_vs_mid"] < s["orca_vs_top"],
            f"slice-plane/orca-{tag}",
            f"mid={s['orca_vs_mid']} top={s['orca_vs_top']}",
        )
    for s in samples:
        if not s["discriminating"]:
            continue
        tag = f"{s['fixture']}-L{s['layer']}"
        expect(
            s["plane_separation"] > 0.10,
            f"slice-plane/sep-{tag}",
            str(s["plane_separation"]),
        )
        expect(
            s["pnp_vs_top"] < 0.02, f"slice-plane/pnp-top-{tag}", str(s["pnp_vs_top"])
        )
        expect(
            s["pnp_vs_mid"] > 0.10, f"slice-plane/pnp-mid-{tag}", str(s["pnp_vs_mid"])
        )
        expect(
            s["orca_vs_mid"] < 0.05,
            f"slice-plane/orca-mid-{tag}",
            str(s["orca_vs_mid"]),
        )
        expect(
            s["orca_vs_top"] > 0.10,
            f"slice-plane/orca-top-{tag}",
            str(s["orca_vs_top"]),
        )
    base0 = next(s for s in samples if s["fixture"] == "base" and s["layer"] == 0)
    expect(
        base0["pnp_outer_wall_hull_mm2"] / base0["orca_outer_wall_hull_mm2"] > 3.0,
        "slice-plane/base0-ratio",
        str(base0["pnp_outer_wall_hull_mm2"] / base0["orca_outer_wall_hull_mm2"]),
    )
    expect(
        base0["stl_top_o200_mm2"] / base0["stl_mid_o210_mm2"] > 3.0,
        "slice-plane/base0-stl-ratio",
        str(base0["stl_top_o200_mm2"] / base0["stl_mid_o210_mm2"]),
    )


def check_wall_width(d):
    base = d["wall_width"]["base"]
    expect(
        base["same_plane_wall_width_effect"] < 0.07,
        "wall-width/base-effect",
        str(base["same_plane_wall_width_effect"]),
    )
    expect(
        base["observed_ratio"] > 3.0,
        "wall-width/base-ratio",
        str(base["observed_ratio"]),
    )
    expect(
        close(base["pnp_hull_mm2"] / base["stl_top_o200_mm2"], 1.0, 0.02),
        "wall-width/base-pnp-fit",
        str(base["pnp_hull_mm2"] / base["stl_top_o200_mm2"]),
    )
    benchy = d["wall_width"]["benchy"]
    expect(
        benchy["same_plane_wall_width_effect"] < 0.01,
        "wall-width/benchy-effect",
        str(benchy["same_plane_wall_width_effect"]),
    )
    expect(
        benchy["observed_ratio"] < 1.05,
        "wall-width/benchy-ratio",
        str(benchy["observed_ratio"]),
    )


def check_anchor(d):
    a = d["anchor"]
    expect(a["loops"] == 22, "anchor/loops", str(a["loops"]))
    expect(a["observed_delta_mm"] > 50, "anchor/observed", str(a["observed_delta_mm"]))
    expect(
        a["slice_plane_term_mm"] > 80,
        "anchor/plane-term",
        str(a["slice_plane_term_mm"]),
    )
    expect(a["anchor_term_mm"] < -20, "anchor/anchor-term", str(a["anchor_term_mm"]))
    expect(a["residual_rel"] < 0.10, "anchor/residual", str(a["residual_rel"]))
    expect(
        close(
            a["observed_delta_mm"], a["pnp_brim_path_mm"] - a["orca_brim_path_mm"], 0.01
        ),
        "anchor/delta-definition",
        str(a["observed_delta_mm"] - (a["pnp_brim_path_mm"] - a["orca_brim_path_mm"])),
    )


def check_brim(d):
    b = d["brim"]
    base_off = b["base-classic-off"]
    expect(
        base_off["brim_swept_rel_diff"] < 0.12,
        "brim/base-off-swept",
        str(base_off["brim_swept_rel_diff"]),
    )
    expect(
        base_off["skirt_outer_loop_rel_diff"] > 0.03,
        "brim/base-off-skirt",
        str(base_off["skirt_outer_loop_rel_diff"]),
    )
    expect(
        base_off["pnp"]["runs"] == 22 and base_off["orca"]["runs"] == 22,
        "brim/base-off-runs",
        f"{base_off['pnp']['runs']}/{base_off['orca']['runs']}",
    )
    expect(
        base_off["pnp"]["inner_loop_mm2"] > base_off["orca"]["inner_loop_mm2"] * 1.2,
        "brim/base-off-inner",
        f"{base_off['pnp']['inner_loop_mm2']}/{base_off['orca']['inner_loop_mm2']}",
    )

    benchy_off = b["benchy-classic-off"]
    expect(
        benchy_off["brim_swept_rel_diff"] < 0.12
        and benchy_off["skirt_outer_loop_rel_diff"] < 0.03,
        "brim/benchy-off-pass",
        f"{benchy_off['brim_swept_rel_diff']}/{benchy_off['skirt_outer_loop_rel_diff']}",
    )

    benchy_on = b["benchy-classic-on"]
    expect(
        benchy_on["brim_swept_rel_diff"] > 0.40,
        "brim/benchy-on-swept",
        str(benchy_on["brim_swept_rel_diff"]),
    )
    expect(
        benchy_on["skirt_outer_loop_rel_diff"] > 0.03,
        "brim/benchy-on-skirt",
        str(benchy_on["skirt_outer_loop_rel_diff"]),
    )
    expect(
        benchy_on["pnp"]["runs"] in (18, 19) and benchy_on["orca"]["runs"] == 39,
        "brim/benchy-on-runs",
        f"{benchy_on['pnp']['runs']}/{benchy_on['orca']['runs']}",
    )

    for cell in ("base-classic-on", "base-arachne-on"):
        c = b[cell]
        expect(
            c["pnp"]["runs"] == 0 and c["orca"]["runs"] == 0,
            f"brim/{cell}-empty",
            f"{c['pnp']['runs']}/{c['orca']['runs']}",
        )
        expect(
            c["skirt_outer_loop_rel_diff"] > 0.10,
            f"brim/{cell}-skirt",
            str(c["skirt_outer_loop_rel_diff"]),
        )


def check_supports(d):
    for cell in (
        "base-classic-on",
        "base-arachne-on",
        "benchy-classic-on",
        "benchy-arachne-on",
    ):
        s = d["supports_on"][cell]
        expect(
            s["support_path_ratio"] > 10.0,
            f"supports/{cell}-path-ratio",
            str(s["support_path_ratio"]),
        )
        cov_ratio = s["orca_support"]["coverage_mm2"] / s["pnp_support"]["coverage_mm2"]
        expect(cov_ratio > 8.0, f"supports/{cell}-coverage-ratio", str(cov_ratio))
        expect(
            abs(s["union_hull_ratio"] - s["skirt_ratio"]) < 0.05,
            f"supports/{cell}-skirt-tracks-input",
            f"union={s['union_hull_ratio']} skirt={s['skirt_ratio']}",
        )
    for cell in (
        "base-classic-off",
        "base-arachne-off",
        "benchy-classic-off",
        "benchy-arachne-off",
    ):
        s = d["supports_off"][cell]
        expect(
            abs(s["union_hull_ratio"] - s["skirt_ratio"]) < 0.06,
            f"supports-off/{cell}-skirt-tracks-input",
            f"union={s['union_hull_ratio']} skirt={s['skirt_ratio']}",
        )
    base_on = d["supports_on"]["base-classic-on"]
    expect(
        abs(base_on["union_hull_ratio"] - 1.1528) < 1e-3,
        "supports/base-on-union",
        str(base_on["union_hull_ratio"]),
    )
    benchy_on = d["supports_on"]["benchy-classic-on"]
    expect(
        abs(benchy_on["union_hull_ratio"] - 0.9567) < 1e-3,
        "supports/benchy-on-union",
        str(benchy_on["union_hull_ratio"]),
    )


def check_oracle(d):
    o = d["oracle"]
    expect(o["cell_count"] == 8, "oracle/cells", str(o["cell_count"]))
    expect(o["pass_count"] == 2, "oracle/pass-count", str(o["pass_count"]))
    cells = o["cells"]
    expect(
        cells["benchy-classic-off"]["pass"] and cells["benchy-arachne-off"]["pass"],
        "oracle/benchy-off-pass",
        json.dumps({k: cells[k]["pass"] for k in cells}),
    )
    base_off = cells["base-classic-off"]
    expect(not base_off["pass"], "oracle/base-off-fails", str(base_off["pass"]))
    expect(
        base_off["brim_swept_area_rel_diff"] < 0.12,
        "oracle/base-off-brim-inside",
        str(base_off["brim_swept_area_rel_diff"]),
    )
    expect(
        base_off["skirt_outer_loop_area_rel_diff"] > 0.03,
        "oracle/base-off-skirt-outside",
        str(base_off["skirt_outer_loop_area_rel_diff"]),
    )
    benchy_on = cells["benchy-classic-on"]
    expect(
        benchy_on["brim_swept_area_rel_diff"] > 0.40,
        "oracle/benchy-on-brim-outside",
        str(benchy_on["brim_swept_area_rel_diff"]),
    )
    base_on = cells["base-classic-on"]
    expect(
        base_on["brim_swept_area_rel_diff"] == 0.0 and base_on["pnp_brim_loops"] == 0,
        "oracle/base-on-empty-brim",
        json.dumps(base_on),
    )
    expect(
        base_on["skirt_outer_loop_area_rel_diff"] > 0.10,
        "oracle/base-on-skirt-outside",
        str(base_on["skirt_outer_loop_area_rel_diff"]),
    )


def check_price(d):
    p = d["price"]
    expect(
        p["layer0_plane_ratio"]["base"] > 2.0,
        "price/base-layer0-ratio",
        str(p["layer0_plane_ratio"]["base"]),
    )
    expect(
        p["layer0_plane_ratio"]["benchy"] < 1.05,
        "price/benchy-layer0-ratio",
        str(p["layer0_plane_ratio"]["benchy"]),
    )
    expect(
        p["layer_fraction"]["base"] < 0.01 and p["layer_fraction"]["benchy"] < 0.01,
        "price/layer-fraction",
        json.dumps(p["layer_fraction"]),
    )
    expect(
        p["required_cut_pct_range"] == [59.1, 73.5],
        "price/required-cut",
        str(p["required_cut_pct_range"]),
    )


CHECKS = [
    ("layer-counts", check_layer_counts),
    ("slice-plane", check_slice_plane),
    ("wall-width", check_wall_width),
    ("anchor", check_anchor),
    ("brim", check_brim),
    ("supports", check_supports),
    ("oracle", check_oracle),
    ("price", check_price),
]


def run_checks(d):
    for name, fn in CHECKS:
        fn(d)
    return len(CHECKS)


MUTATIONS = [
    (
        "slice-plane swap",
        lambda d: d["slice_plane"]["samples"][5].__setitem__(
            "pnp_outer_wall_hull_mm2",
            d["slice_plane"]["samples"][5]["orca_outer_wall_hull_mm2"],
        ),
    ),
    (
        "slice-plane separation",
        lambda d: d["slice_plane"]["samples"][5].__setitem__("plane_separation", 0.0),
    ),
    (
        "wall-width effect",
        lambda d: d["wall_width"]["base"].__setitem__(
            "same_plane_wall_width_effect", 0.30
        ),
    ),
    (
        "anchor sign",
        lambda d: d["anchor"].__setitem__(
            "observed_delta_mm", -d["anchor"]["observed_delta_mm"]
        ),
    ),
    (
        "base-off brim",
        lambda d: d["brim"]["base-classic-off"].__setitem__(
            "brim_swept_rel_diff", 0.20
        ),
    ),
    (
        "benchy-off skirt",
        lambda d: d["brim"]["benchy-classic-off"].__setitem__(
            "skirt_outer_loop_rel_diff", 0.05
        ),
    ),
    (
        "benchy-on support ratio",
        lambda d: d["supports_on"]["benchy-classic-on"].__setitem__(
            "support_path_ratio", 1.0
        ),
    ),
    (
        "base-on union",
        lambda d: d["supports_on"]["base-classic-on"].__setitem__(
            "union_hull_ratio", 1.0
        ),
    ),
    ("oracle pass count", lambda d: d["oracle"].__setitem__("pass_count", 8)),
    (
        "base layer count",
        lambda d: d["layer_counts"]["base"].__setitem__("pnp_layers", 496),
    ),
    (
        "price layer0 ratio",
        lambda d: d["price"]["layer0_plane_ratio"].__setitem__("base", 1.0),
    ),
    ("anchor residual", lambda d: d["anchor"].__setitem__("residual_rel", 0.50)),
]


def main() -> int:
    if not SUMMARY.is_file():
        print(f"missing {SUMMARY}", file=sys.stderr)
        return 2
    data = json.loads(SUMMARY.read_text(encoding="utf-8"))

    if "--self-test" in sys.argv:
        rejected = 0
        for name, mutate in MUTATIONS:
            trial = copy.deepcopy(data)
            mutate(trial)
            try:
                run_checks(trial)
            except CheckFailure:
                rejected += 1
            else:
                print(f"SELF-TEST FAIL: mutation not rejected: {name}")
        print(f"SELF-TEST: {rejected}/{len(MUTATIONS)} negative controls rejected")
        return 0 if rejected == len(MUTATIONS) else 1

    try:
        run_checks(data)
    except CheckFailure as failure:
        print(f"FAIL: {failure}")
        return 1
    print(f"PASS: {len(CHECKS)} claim groups re-derived from reduced/summary.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
