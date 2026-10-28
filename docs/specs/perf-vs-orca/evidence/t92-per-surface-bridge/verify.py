#!/usr/bin/env python3
"""t92 Phase-1 screening verifier.

Re-derives the screening table and the measured-dead verdict from the committed
reduced observation (`probe-rows.json`), the committed post-t81 reference set
(`t91-exact-z-memo/references.json`) and the committed probe patch. Exits 0
against a plain checkout — the `.local-artifacts` capture tree is optional and
never read here.

`--self-test` runs the same checks against deliberately corrupted copies and
requires every negative control to be rejected.
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVIDENCE = HERE.parent
ROWS_PATH = HERE / "probe-rows.json"
PATCH_PATH = HERE / "probe.patch"
REFERENCES_PATH = EVIDENCE / "t91-exact-z-memo" / "references.json"

CELL = "ordinary-base-classic-off"
CELL_WALL_SECONDS = (
    73.17  # t87 scoreboard, the cell the ticket's floor is priced against
)
FLOOR_FRACTION = 0.008  # the map's 0.80 % screening floor
TOL_MS = 1.0


def recompute(rows: dict) -> dict:
    summary = rows["probe_summary"]
    anchors = rows["probe_anchors"]
    ratio = rows["cpu_wall_ratio"]
    anchors_total = anchors["total_fill_offset_ms"] + anchors["limiting_offset_ms"]
    per_surface = (
        summary["surface_offset_ms"]
        + summary["surface_clip_ms"]
        + summary["qualify_ms"]
        + summary["retain_ms"]
    )
    per_candidate = summary["angles_ms"] - anchors_total
    ceiling_ms = per_surface + per_candidate + anchors["total_fill_offset_ms"]
    whole_layer_ms = summary["deep_clip_offset_ms"] + summary["unsupported_offset_ms"]
    return {
        "ratio": ratio,
        "per_surface_ms": per_surface,
        "per_candidate_ms": per_candidate,
        "hoistable_anchor_ms": anchors["total_fill_offset_ms"],
        "ticket_ceiling_ms": ceiling_ms,
        "ticket_ceiling_wall_ms": ceiling_ms / ratio,
        "ticket_ceiling_fraction": ceiling_ms / ratio / 1000.0 / CELL_WALL_SECONDS,
        "whole_layer_offsets_ms": whole_layer_ms,
        "whole_layer_wall_ms": whole_layer_ms / ratio,
        "whole_layer_fraction": whole_layer_ms / ratio / 1000.0 / CELL_WALL_SECONDS,
    }


def check(rows: dict, patch_text: str, references: dict) -> list[str]:
    failures: list[str] = []

    def fail(code: str, detail: str) -> None:
        failures.append(f"{code}: {detail}")

    summary = rows.get("probe_summary") or {}
    anchors = rows.get("probe_anchors") or {}
    layers = rows.get("layer_rows") or []
    angles_rows = rows.get("angles_rows") or []
    stages = rows.get("prepass_stages_ms") or {}
    ratio = rows.get("cpu_wall_ratio", 0.0)

    # 1. Output identity against the committed post-t81 reference set.
    ref = (references.get("cells") or {}).get(CELL) or {}
    if rows.get("output_sha256") != ref.get("sha256"):
        fail(
            "E_SHA",
            f"output sha {rows.get('output_sha256')!r} != reference {ref.get('sha256')!r}",
        )
    if rows.get("reference_sha256") != ref.get("sha256"):
        fail(
            "E_SHA_REF", "the recorded reference sha is not the committed reference sha"
        )

    # 2. Wall / CPU / ratio internal consistency.
    wall = rows.get("wall_seconds", 0.0)
    cpu = rows.get("cpu_seconds", 0.0)
    if wall <= 0 or cpu <= 0 or ratio <= 0:
        fail("E_RATIO", "wall, cpu and ratio must be positive")
    elif abs(cpu / wall - ratio) > 1e-3:
        fail("E_RATIO", f"cpu/wall {cpu / wall:.4f} != recorded ratio {ratio}")

    # 3. terms_ms mirrors the probe summary; wall equivalents use the run ratio.
    terms = rows.get("terms_ms") or {}
    derived_terms = {
        "deep_clip_offset": summary.get("deep_clip_offset_ms"),
        "surface_offset": summary.get("surface_offset_ms"),
        "surface_clip": summary.get("surface_clip_ms"),
        "qualify": summary.get("qualify_ms"),
        "unsupported_offset": summary.get("unsupported_offset_ms"),
        "retain": summary.get("retain_ms"),
        "expansion": summary.get("expansion_ms"),
        "angles": summary.get("angles_ms"),
        "anchor_total_fill_hoistable": anchors.get("total_fill_offset_ms"),
        "anchor_limiting": anchors.get("limiting_offset_ms"),
    }
    for name, value in derived_terms.items():
        if value is None:
            fail("E_TERMS", f"missing probe term {name}")
        elif abs(terms.get(name, float("nan")) - value) > 1e-6:
            fail("E_TERMS", f"terms_ms[{name}] {terms.get(name)} != probe {value}")
    for name, value in terms.items():
        expected = value / ratio
        recorded = (rows.get("terms_wall_equivalent_ms") or {}).get(name)
        if recorded is None or abs(recorded - expected) > 1e-3:
            fail("E_WALL_EQ", f"wall-equivalent[{name}] {recorded} != {expected:.3f}")

    # 4. Per-layer rows reconcile with the aggregate counters.
    if len(layers) != summary.get("calls"):
        fail(
            "E_LAYER_COUNT", f"{len(layers)} layer rows != calls {summary.get('calls')}"
        )
    if sum(row.get("surfaces", 0) for row in layers) != summary.get("surfaces"):
        fail("E_LAYER_SUM", "layer surfaces do not sum to the probe total")
    if sum(row.get("deep_clip_polys", 0) for row in layers) != summary.get(
        "deep_clip_polys"
    ):
        fail("E_LAYER_SUM", "layer deep_clip_polys do not sum to the probe total")
    for term in (
        "deep_clip_offset_ms",
        "surface_offset_ms",
        "surface_clip_ms",
        "qualify_ms",
        "unsupported_offset_ms",
        "retain_ms",
    ):
        total = sum(row.get(term, 0.0) for row in layers)
        expected = summary.get(
            "deep_clip_offset_ms" if term == "deep_clip_offset_ms" else term
        )
        if term == "deep_clip_offset_ms":
            expected = summary.get("deep_clip_offset_ms")
        if abs(total - expected) > max(TOL_MS, 0.0015 * len(layers)):
            fail(
                "E_LAYER_SUM",
                f"layer {term} sums to {total:.3f}, probe reports {expected}",
            )
    for row in layers:
        own = sum(
            row.get(term, 0.0)
            for term in (
                "deep_clip_offset_ms",
                "surface_offset_ms",
                "surface_clip_ms",
                "qualify_ms",
                "unsupported_offset_ms",
                "retain_ms",
            )
        )
        if own > row.get("total_ms", 0.0) + 0.05:
            fail(
                "E_LAYER_SPAN",
                f"layer {row.get('layer')} terms {own:.3f} > span {row.get('total_ms')}",
            )

    # 5. Angle calls reconcile with the qualified-call counter and the anchors.
    if len(angles_rows) != summary.get("qualified_calls"):
        fail(
            "E_ANGLES_COUNT",
            f"{len(angles_rows)} angle rows != qualified_calls {summary.get('qualified_calls')}",
        )
    angle_sum = sum(row.get("angles_ms", 0.0) for row in angles_rows)
    if abs(angle_sum - summary.get("angles_ms", 0.0)) > max(
        TOL_MS, 0.0015 * len(angles_rows)
    ):
        fail(
            "E_ANGLES_SUM",
            f"angle rows sum to {angle_sum:.3f}, probe reports {summary.get('angles_ms')}",
        )
    candidates = sum(row.get("candidates", 0) for row in angles_rows)
    if anchors.get("calls") != 2 * candidates:
        fail(
            "E_ANCHOR_CALLS",
            f"anchor offset calls {anchors.get('calls')} != 2 x candidates {candidates}",
        )

    # 6. The stage bracket the screen is priced inside is present.
    if stages.get("PrePass::ShellClassification", 0) <= 0:
        fail("E_BRACKET", "PrePass::ShellClassification stage wall missing")

    # 7. The probe patch carries both env-gated sites, and the screening verdict
    #    is re-derived, not asserted: the ticket's ceiling is below the floor
    #    while the whole-layer offset bracket is above it.
    if "PNP_T92_PROBE" not in patch_text or "T92PROBE" not in patch_text:
        fail("E_PATCH", "probe.patch does not carry the env-gated probe")
    for needle in ("bridge_over_infill.rs", "slice_postprocess_prepass.rs"):
        if needle not in patch_text:
            fail("E_PATCH", f"probe.patch does not touch {needle}")
    derived = recompute(rows)
    if not derived["ticket_ceiling_fraction"] < FLOOR_FRACTION:
        fail(
            "E_CEILING_FLOOR",
            f"ticket ceiling {derived['ticket_ceiling_fraction']:.4%} is not below the "
            f"{FLOOR_FRACTION:.2%} floor",
        )
    if not derived["ticket_ceiling_wall_ms"] > 0:
        fail("E_CEILING_FLOOR", "ticket ceiling is not positive")
    if not derived["whole_layer_fraction"] > FLOOR_FRACTION:
        fail(
            "E_WHOLE_LAYER",
            f"whole-layer offset bracket {derived['whole_layer_fraction']:.4%} is not above the floor",
        )
    return failures


CONTROLS = {
    "output sha swapped": ("E_SHA", lambda r: r.__setitem__("output_sha256", "0" * 64)),
    "reference sha rewritten": (
        "E_SHA_REF",
        lambda r: r.__setitem__("reference_sha256", "1" * 64),
    ),
    "ratio inflated": ("E_RATIO", lambda r: r.__setitem__("cpu_wall_ratio", 9.7)),
    "surface clip term shrunk": (
        "E_TERMS",
        lambda r: r["terms_ms"].__setitem__("surface_clip", 0.0),
    ),
    "wall equivalent stale": (
        "E_WALL_EQ",
        lambda r: r["terms_wall_equivalent_ms"].__setitem__("angles", 1.0),
    ),
    "layer row dropped": ("E_LAYER_COUNT", lambda r: r["layer_rows"].pop()),
    "layer surfaces understated": (
        "E_LAYER_SUM",
        lambda r: r["layer_rows"][0].__setitem__("surfaces", 0),
    ),
    "layer term exceeds its span": (
        "E_LAYER_SPAN",
        lambda r: r["layer_rows"][0].__setitem__("total_ms", 0.0),
    ),
    "angle row dropped": ("E_ANGLES_COUNT", lambda r: r["angles_rows"].pop()),
    "angle sum understated": (
        "E_ANGLES_SUM",
        lambda r: r["angles_rows"][0].__setitem__("angles_ms", 0.0),
    ),
    "anchor call count doubled": (
        "E_ANCHOR_CALLS",
        lambda r: r["probe_anchors"].__setitem__("calls", 72),
    ),
    "stage bracket removed": (
        "E_BRACKET",
        lambda r: r["prepass_stages_ms"].pop("PrePass::ShellClassification"),
    ),
    "ceiling pushed over the floor": (
        "E_CEILING_FLOOR",
        lambda r: r["probe_summary"].__setitem__("angles_ms", 100000.0),
    ),
    "whole-layer bracket pushed below the floor": (
        "E_WHOLE_LAYER",
        lambda r: (
            r["probe_summary"].__setitem__("deep_clip_offset_ms", 0.0),
            r["probe_summary"].__setitem__("unsupported_offset_ms", 0.0),
        ),
    ),
}


def load() -> tuple[dict, str, dict]:
    rows = json.loads(ROWS_PATH.read_text(encoding="utf-8"))
    patch_text = PATCH_PATH.read_text(encoding="utf-8")
    references = json.loads(REFERENCES_PATH.read_text(encoding="utf-8"))
    return rows, patch_text, references


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    rows, patch_text, references = load()

    failures = check(rows, patch_text, references)
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 1
    derived = recompute(rows)
    print(
        f"ticket ceiling         {derived['ticket_ceiling_ms']:.1f} ms accumulated "
        f"= {derived['ticket_ceiling_wall_ms']:.1f} ms wall "
        f"= {derived['ticket_ceiling_fraction']:.4%} of {CELL_WALL_SECONDS} s"
    )
    print(
        f"whole-layer offsets    {derived['whole_layer_offsets_ms']:.1f} ms accumulated "
        f"= {derived['whole_layer_wall_ms']:.1f} ms wall "
        f"= {derived['whole_layer_fraction']:.4%} of {CELL_WALL_SECONDS} s"
    )
    print(f"screening floor        {FLOOR_FRACTION:.2%}")

    if args.self_test:
        rejected = 0
        problems: list[str] = []
        if "PNP_T92_PROBE" not in patch_text:
            problems.append("E_PATCH: shipped probe.patch lacks the env gate")
        else:
            broken_patch = patch_text.replace("PNP_T92_PROBE", "PNP_DISABLED")
            if not any(
                f.startswith("E_PATCH") for f in check(rows, broken_patch, references)
            ):
                problems.append("E_PATCH: corrupted patch control was accepted")
            rejected += 1
        for name, (code, mutate) in CONTROLS.items():
            corrupted = copy.deepcopy(rows)
            mutate(corrupted)
            codes = check(corrupted, patch_text, references)
            if not any(f.startswith(code) for f in codes):
                problems.append(f"{code}: control {name!r} was accepted (got {codes})")
            else:
                rejected += 1
        total = len(CONTROLS) + 1
        if problems:
            for problem in problems:
                print(f"FAIL {problem}")
            return 1
        print(f"self-test              {rejected}/{total} negative controls rejected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
