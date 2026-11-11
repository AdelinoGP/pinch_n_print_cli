#!/usr/bin/env python3
"""Ticket 139 verifier: every claim re-derived from `reduced/delta.json` only.

No raw G-code, no STL, no campaign tree. `--self-test` mutates the summary and
asserts each mutation is rejected, so a broken verifier cannot pass vacuously.

Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/verify.py
    python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

SUMMARY = Path(__file__).resolve().parent / "reduced" / "delta.json"
CAMPAIGN = Path(__file__).resolve().parent / "reduced" / "campaign.json"
SCOREBOARD = Path(__file__).resolve().parent / "reduced" / "scoreboard.json"

# The recorded adhesion-oracle verdict: the two supports-off benchy cells pass;
# base supports-off fails the 3 % skirt band on the anchor/contour residual and
# the four supports-on cells are support-engine-limited.
ORACLE_EXPECTED = {
    "benchy-classic-off": [],
    "benchy-arachne-off": [],
    "benchy-classic-on": ["brim_swept_area_pass", "pass"],
    "benchy-arachne-on": ["brim_swept_area_pass", "pass"],
    "base-classic-off": ["pass", "skirt_area_pass"],
    "base-arachne-off": ["pass", "skirt_area_pass"],
    "base-classic-on": ["pass", "skirt_growth_pass"],
    "base-arachne-on": ["pass", "skirt_growth_pass"],
}

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


def rel(value: float, reference: float) -> float:
    if reference == 0:
        return float("inf") if value else 0.0
    return abs(value - reference) / reference


def check(summary: dict) -> list[str]:
    errors: list[str] = []
    cells = summary.get("cells", {})
    rows = summary.get("rows", {})
    if sorted(cells) != sorted(CELLS):
        errors.append(f"cell set drifted: {sorted(cells)}")
        return errors
    if sorted(rows) != sorted(CELLS):
        errors.append(f"row set drifted: {sorted(rows)}")

    # Claim 1: the baseline arm reproduces the t135 Stage-1 reference set, the
    # candidate arms agree with each other, and the candidate set moved.
    for cell in CELLS:
        row = rows.get(cell, {})
        if not row.get("baseline_matches_t135_reference"):
            errors.append(
                f"{cell}: baseline does not reproduce the t135 reference hash"
            )
        if not row.get("candidate_arms_identical"):
            errors.append(f"{cell}: ordinary and accelerated candidate hashes differ")
        if row.get("t135_reference_sha256") == row.get("candidate_ordinary_sha256"):
            errors.append(f"{cell}: candidate hash did not move off the t135 reference")
        if row.get("candidate_ordinary_sha256") != row.get(
            "candidate_ordinary_sha256_on_disk"
        ):
            errors.append(
                f"{cell}: candidate ordinary disk hash disagrees with rows.csv"
            )
        if row.get("candidate_accelerated_sha256") != row.get(
            "candidate_accelerated_sha256_on_disk"
        ):
            errors.append(
                f"{cell}: candidate accelerated disk hash disagrees with rows.csv"
            )

    # Claim 2: the layer grid gained the canonical final layer on base only.
    for cell in CELLS:
        rec = cells[cell]
        if cell.startswith("base-"):
            if (rec["baseline_layers"], rec["candidate_layers"]) != (495, 496):
                errors.append(
                    f"{cell}: base layer count {rec['baseline_layers']}->{rec['candidate_layers']}"
                )
            if (rec["baseline_last_z"], rec["candidate_last_z"]) != (99.0, 99.2):
                errors.append(
                    f"{cell}: base last Z {rec['baseline_last_z']}->{rec['candidate_last_z']}"
                )
        else:
            if (
                rec["baseline_layers"] != rec["candidate_layers"]
                or rec["candidate_layers"] != 240
            ):
                errors.append(
                    f"{cell}: benchy layer count {rec['baseline_layers']}->{rec['candidate_layers']}"
                )
            if rec["candidate_last_z"] != rec["baseline_last_z"]:
                errors.append(f"{cell}: benchy last Z moved")

    # Claim 3: no common layer was dropped and the feature inventory is stable.
    for cell in CELLS:
        rec = cells[cell]
        if rec["common_layers_missing_from_candidate"]:
            errors.append(
                f"{cell}: dropped common layers {rec['common_layers_missing_from_candidate']}"
            )
        if not rec["types_equal"]:
            errors.append(f"{cell}: feature inventory changed")

    # Claim 4: the layer-0 outer wall now fits the STL midpoint better than the
    # top on every cell, and base's steep foot is a >2x discriminator.
    for cell in CELLS:
        rec = cells[cell]
        hull = rec["candidate_l0_outer_wall_hull_mm2"]
        mid = rec["stl_mid_section_hull_o200_mm2"]
        top = rec["stl_top_section_hull_o200_mm2"]
        if rel(hull, mid) >= rel(hull, top):
            errors.append(
                f"{cell}: hull {hull} is not closer to the mid section {mid} than the top {top}"
            )
        if rec["fixture"].endswith("base.stl"):
            separation = abs(top - mid) / min(top, mid)
            if separation < 2.0:
                errors.append(
                    f"{cell}: base top/mid separation {separation:.2f} below the 2x discriminator"
                )
    return errors


def check_campaign(campaign: dict) -> list[str]:
    errors: list[str] = []
    if not campaign.get("candidate_arms_identical"):
        errors.append("campaign: candidate arms are not byte-identical")
    reference = campaign.get("reference_set", {})
    if sorted(reference) != sorted(CELLS):
        errors.append(f"campaign: reference-set cells drifted: {sorted(reference)}")
    ab = campaign.get("ab", {})
    if sorted(ab) != sorted(CELLS):
        errors.append(f"campaign: A/B cells drifted: {sorted(ab)}")
    for cell in CELLS:
        row = ab.get(cell, {})
        if row.get("pairs") != 3:
            errors.append(f"{cell}: A/B pair count {row.get('pairs')} != 3")
        if row.get("wall_ratio_candidate_over_baseline", 9.0) > 1.0:
            errors.append(f"{cell}: candidate wall is slower than the baseline")
        if row.get("cpu_ratio_candidate_over_baseline", 9.0) > 1.0:
            errors.append(f"{cell}: candidate CPU exceeds the baseline")
    oracle = campaign.get("adhesion_oracle", {})
    cells = oracle.get("cells", {})
    if sorted(cells) != sorted(CELLS):
        errors.append(f"campaign: oracle cells drifted: {sorted(cells)}")
    for cell in CELLS:
        rec = cells.get(cell, {})
        expected = ORACLE_EXPECTED[cell]
        if sorted(rec.get("failing_checks", [])) != sorted(expected):
            errors.append(f"{cell}: oracle failing gates {rec.get('failing_checks')} != {expected}")
        if rec.get("pass") != (not expected):
            errors.append(f"{cell}: oracle pass flag contradicts its failing gates")
    if oracle.get("pass_count") != 2:
        errors.append(f"campaign: oracle pass count {oracle.get('pass_count')} != 2")
    return errors


def check_scoreboard(scoreboard: dict) -> list[str]:
    errors: list[str] = []
    cells = scoreboard.get("cells", {})
    if sorted(cells) != sorted(CELLS):
        errors.append(f"scoreboard: cells drifted: {sorted(cells)}")
        return errors
    for cell in CELLS:
        rec = cells[cell]
        for tool in ("orca", "pnp-ordinary", "pnp-accelerated"):
            row = rec.get(tool, {})
            if row.get("runs") != 3:
                errors.append(f"{cell}/{tool}: {row.get('runs')} measured runs, expected 3")
            if len(row.get("wall_runs_s", [])) != 3:
                errors.append(f"{cell}/{tool}: wall run list is not 3 long")
        if rec["pnp-ordinary"]["gcode_sha256"] != rec["pnp-accelerated"]["gcode_sha256"]:
            errors.append(f"{cell}: pnp build arms emitted different bytes")
        for tool in ("pnp-ordinary", "pnp-accelerated"):
            if rec[tool]["ratio_vs_orca"] <= 1.0:
                errors.append(f"{cell}/{tool}: ratio {rec[tool]['ratio_vs_orca']} claims a win")
            expected_cut = (1.0 - 1.0 / rec[tool]["ratio_vs_orca"]) * 100.0
            if abs(rec[tool]["required_cut_pct"] - expected_cut) > 0.05:
                errors.append(f"{cell}/{tool}: required cut is not 1 - 1/ratio")
    for mode, lo, hi in (
        ("pnp-ordinary", 2.279, 4.187),
        ("pnp-accelerated", 2.311, 4.179),
    ):
        ratios = [cells[cell][mode]["ratio_vs_orca"] for cell in CELLS]
        if round(min(ratios), 3) != lo or round(max(ratios), 3) != hi:
            errors.append(f"scoreboard: {mode} ratio range {min(ratios):.3f}-{max(ratios):.3f} != {lo}-{hi}")
    return errors


def main(argv: list[str]) -> int:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    campaign = json.loads(CAMPAIGN.read_text(encoding="utf-8"))
    scoreboard = json.loads(SCOREBOARD.read_text(encoding="utf-8"))
    errors = check(summary) + check_campaign(campaign) + check_scoreboard(scoreboard)
    if errors:
        for error in errors:
            print(f"FAIL {error}")
        return 1
    print(f"PASS {len(CELLS)} cells, 7 claim groups")

    if "--self-test" in argv:
        mutations = []

        def mutate(name, fn):
            data = copy.deepcopy(summary)
            fn(data)
            mutations.append((name, data))

        mutate(
            "baseline hash",
            lambda d: d["rows"][CELLS[0]].__setitem__(
                "baseline_matches_t135_reference", False
            ),
        )
        mutate(
            "arm divergence",
            lambda d: d["rows"][CELLS[1]].__setitem__(
                "candidate_arms_identical", False
            ),
        )
        mutate(
            "candidate unmoved",
            lambda d: d["rows"][CELLS[2]].__setitem__(
                "candidate_ordinary_sha256",
                d["rows"][CELLS[2]]["t135_reference_sha256"],
            ),
        )
        mutate(
            "disk hash",
            lambda d: d["rows"][CELLS[3]].__setitem__(
                "candidate_ordinary_sha256_on_disk", "0" * 64
            ),
        )
        mutate(
            "base layers",
            lambda d: d["cells"][CELLS[4]].__setitem__("candidate_layers", 495),
        )
        mutate(
            "base last z",
            lambda d: d["cells"][CELLS[5]].__setitem__("candidate_last_z", 99.0),
        )
        mutate(
            "benchy layers",
            lambda d: d["cells"][CELLS[0]].__setitem__("candidate_layers", 241),
        )
        mutate(
            "missing layer",
            lambda d: d["cells"][CELLS[6]].__setitem__(
                "common_layers_missing_from_candidate", [69.4]
            ),
        )
        mutate(
            "feature drift",
            lambda d: d["cells"][CELLS[7]].__setitem__("types_equal", False),
        )
        mutate(
            "hull not mid",
            lambda d: d["cells"][CELLS[4]].__setitem__(
                "candidate_l0_outer_wall_hull_mm2",
                d["cells"][CELLS[4]]["stl_top_section_hull_o200_mm2"],
            ),
        )
        mutate(
            "weak base discriminator",
            lambda d: (
                d["cells"][CELLS[4]].__setitem__("stl_top_section_hull_o200_mm2", 1.0),
                d["cells"][CELLS[4]].__setitem__("stl_mid_section_hull_o200_mm2", 0.9),
            ),
        )
        mutate("cell set", lambda d: d["cells"].pop(CELLS[0]))

        campaign_mutations = []
        def cmutate(name, fn):
            data = copy.deepcopy(campaign)
            fn(data)
            campaign_mutations.append((name, data))

        cmutate("arms diverged", lambda d: d.__setitem__("candidate_arms_identical", False))
        cmutate("ab pairs", lambda d: d["ab"][CELLS[0]].__setitem__("pairs", 2))
        cmutate("candidate slower", lambda d: d["ab"][CELLS[0]].__setitem__("wall_ratio_candidate_over_baseline", 1.2))
        cmutate("oracle gate", lambda d: d["adhesion_oracle"]["cells"][CELLS[4]].__setitem__("failing_checks", []))
        cmutate("oracle pass count", lambda d: d["adhesion_oracle"].__setitem__("pass_count", 3))

        scoreboard_mutations = []
        def smutate(name, fn):
            data = copy.deepcopy(scoreboard)
            fn(data)
            scoreboard_mutations.append((name, data))

        smutate("scoreboard runs", lambda d: d["cells"][CELLS[0]]["orca"].__setitem__("runs", 2))
        smutate("build arms differ", lambda d: d["cells"][CELLS[0]]["pnp-accelerated"].__setitem__("gcode_sha256", "0" * 64))
        smutate("claimed win", lambda d: d["cells"][CELLS[0]]["pnp-ordinary"].__setitem__("ratio_vs_orca", 0.9))
        smutate("bad cut", lambda d: d["cells"][CELLS[1]]["pnp-ordinary"].__setitem__("required_cut_pct", 50.0))
        smutate("ratio range", lambda d: d["cells"][CELLS[2]]["pnp-accelerated"].__setitem__("ratio_vs_orca", 9.0))

        rejected = 0
        for name, data in mutations:
            if check(data):
                rejected += 1
            else:
                print(f"SELF-TEST FAIL: mutation '{name}' was accepted")
        for name, data in campaign_mutations:
            if check_campaign(data):
                rejected += 1
            else:
                print(f"SELF-TEST FAIL: mutation '{name}' was accepted")
        for name, data in scoreboard_mutations:
            if check_scoreboard(data):
                rejected += 1
            else:
                print(f"SELF-TEST FAIL: mutation '{name}' was accepted")
        total = len(mutations) + len(campaign_mutations) + len(scoreboard_mutations)
        print(f"self-test: rejected {rejected}/{total} negative controls")
        return 0 if rejected == total else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
