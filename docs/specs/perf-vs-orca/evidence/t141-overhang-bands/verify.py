#!/usr/bin/env python3
"""Verify the t141 overhang-band claim set.

Checks:
  1. probe-row arithmetic (offsets = 3 x layers, clips = 6 x layers, and the
     measured band thread-time converts to the stage wall at 7x-13x);
  2. the stage's share of the cell wall and of the cell's required cut, on the
     cells timed in-session;
  3. the source reader census: the band partition exists where this ticket
     says it does, the classifier's speed loop gates on the *presence* bit, its
     speed comes from the distance, and no production site compares the
     quartile's value.

`--self-test` mutates each input class and asserts the corresponding check
rejects it (negative controls).
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# .../docs/specs/perf-vs-orca/evidence/t141-overhang-bands -> repo root
REPO = HERE.parents[4]

PROBE = HERE / "probe.json"

SOURCES = {
    "overhang_annotation": REPO / "crates/slicer-core/src/algos/overhang_annotation.rs",
    "perimeter_spatial": REPO / "crates/slicer-core/src/perimeter_spatial.rs",
    "classifier": REPO / "modules/core-modules/overhang-classifier-default/src/lib.rs",
}

REQUIRED_SUBSTRINGS = [
    ("overhang_annotation", "fn partition_into_bands("),
    ("overhang_annotation", "let grown_previous = offset("),
    ("overhang_annotation", "difference_ex(&within_threshold, previous)"),
    ("overhang_annotation", "difference_ex(overhang_area, &cumulative[2])"),
    ("classifier", "if point.overhang_quartile.is_some()"),
    ("classifier", "let current_speed = calculate_speed(distance, &sections, base);"),
    (
        "classifier",
        "fn calculate_speed(distance: f32, sections: &[(f32, f32)], original_speed: f32)",
    ),
    (
        "perimeter_spatial",
        "PathAnnotationMode::Planar => context.overhang_quartile(x, y)",
    ),
    (
        "perimeter_spatial",
        "Some(context.signed_distance_to_boundary(x, y) + 0.5 * width)",
    ),
]

# A production comparison on the quartile's *value* would falsify the census.
VALUE_COMPARISON = re.compile(r"overhang_quartile\s*(==|!=|>=|<=|<|>)")


def load_probe() -> dict:
    return json.loads(PROBE.read_text(encoding="utf-8"))


def check_rows(probe: dict) -> list[str]:
    failures: list[str] = []
    for row in probe["stage_rows"]:
        name = row["run"]
        if row["offset_calls"] != 3 * row["band_layers"]:
            failures.append(
                f"{name}: offsets {row['offset_calls']} != 3 x {row['band_layers']}"
            )
        if row["clip_calls"] != 6 * row["band_layers"]:
            failures.append(
                f"{name}: clips {row['clip_calls']} != 6 x {row['band_layers']}"
            )
        if row["band_layers"] != row["overhang_layers"]:
            failures.append(f"{name}: band layers != overhang layers")
        conversion = row["band_thread_ms"] / row["stage_wall_ms"]
        if not (7.0 <= conversion <= 13.0):
            failures.append(
                f"{name}: thread/wall conversion {conversion:.2f}x outside 7x-13x"
            )
    return failures


def check_shares(probe: dict) -> list[str]:
    failures: list[str] = []
    walls = probe["timed_uninstrumented_wall_s"]
    cuts = probe["required_cut_pct_t139_scoreboard"]
    # pre-declared bands, one per timed cell, from the same-session pair
    bands = {
        "base-classic-off": (20.0, 30.0, 30.0, 45.0),
        "base-arachne-off": (20.0, 30.0, 30.0, 45.0),
        "benchy-classic-off": (18.0, 25.0, 30.0, 45.0),
    }
    by_cell: dict[str, list[dict]] = {}
    for row in probe["stage_rows"]:
        by_cell.setdefault(row["cell"], []).append(row)

    for cell, (wall_lo, wall_hi, cut_lo, cut_hi) in bands.items():
        wall = walls[cell]
        cut_s = wall * cuts[cell] / 100.0
        for row in by_cell[cell]:
            stage_s = row["stage_wall_ms"] / 1000.0
            share = 100.0 * stage_s / wall
            cut_share = 100.0 * stage_s / cut_s
            if not (wall_lo <= share <= wall_hi):
                failures.append(
                    f"{row['run']}: stage share {share:.1f}% outside {wall_lo}-{wall_hi}%"
                )
            if not (cut_lo <= cut_share <= cut_hi):
                failures.append(
                    f"{row['run']}: cut share {cut_share:.1f}% outside {cut_lo}-{cut_hi}%"
                )

    # cells measured with the probe but walled from the t139 scoreboard
    ref = probe["reference_wall_s_t139_scoreboard"]
    for cell, (wall_lo, wall_hi, cut_lo, cut_hi) in {
        "base-classic-on": (10.0, 14.0, 13.0, 18.0),
        "benchy-classic-on": (8.0, 12.0, 12.0, 17.0),
    }.items():
        wall = ref[cell]
        cut_s = wall * cuts[cell] / 100.0
        for row in by_cell[cell]:
            stage_s = row["stage_wall_ms"] / 1000.0
            share = 100.0 * stage_s / wall
            cut_share = 100.0 * stage_s / cut_s
            if not (wall_lo <= share <= wall_hi):
                failures.append(
                    f"{row['run']}: stage share {share:.1f}% outside {wall_lo}-{wall_hi}%"
                )
            if not (cut_lo <= cut_share <= cut_hi):
                failures.append(
                    f"{row['run']}: cut share {cut_share:.1f}% outside {cut_lo}-{cut_hi}%"
                )
    return failures


def check_census(sources: dict[str, str]) -> list[str]:
    failures: list[str] = []
    for key, needle in REQUIRED_SUBSTRINGS:
        text = sources.get(key)
        if text is None or needle not in text:
            failures.append(f"census: {key} is missing {needle!r}")
    for key, text in sources.items():
        for match in VALUE_COMPARISON.finditer(text):
            line_start = text.rfind("\n", 0, match.start()) + 1
            line_end = text.find("\n", match.end())
            line = text[line_start : line_end if line_end >= 0 else None].strip()
            failures.append(f"census: {key} compares the quartile value: {line}")
    return failures


def run(probe: dict, sources: dict[str, str]) -> list[str]:
    return check_rows(probe) + check_shares(probe) + check_census(sources)


def self_test(probe: dict, sources: dict[str, str]) -> int:
    controls: list[tuple[str, list[str]]] = []

    mutated = copy.deepcopy(probe)
    mutated["stage_rows"][0]["offset_calls"] += 1
    controls.append(("row-arithmetic", check_rows(mutated)))

    mutated = copy.deepcopy(probe)
    mutated["stage_rows"][0]["stage_wall_ms"] *= 2.0
    controls.append(("stage-share", check_shares(mutated)))

    mutated_sources = dict(sources)
    mutated_sources["classifier"] = mutated_sources["classifier"].replace(
        "if point.overhang_quartile.is_some()", "if point.overhang_quartile == Some(2)"
    )
    controls.append(("value-comparison", check_census(mutated_sources)))

    mutated_sources = dict(sources)
    mutated_sources["classifier"] = mutated_sources["classifier"].replace(
        "let current_speed = calculate_speed(distance, &sections, base);",
        "let current_speed = calculate_speed_from_quartile(point.overhang_quartile, base);",
    )
    controls.append(("missing-distance-consumer", check_census(mutated_sources)))

    ok = True
    for name, failures in controls:
        if not failures:
            print(f"NEGATIVE CONTROL FAILED (accepted): {name}")
            ok = False
        else:
            print(f"negative control rejected: {name}")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    probe = load_probe()
    sources = {key: path.read_text(encoding="utf-8") for key, path in SOURCES.items()}

    if args.self_test:
        return self_test(probe, sources)

    failures = run(probe, sources)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print(f"OK: {len(probe['stage_rows'])} probe rows, {len(SOURCES)} source files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
