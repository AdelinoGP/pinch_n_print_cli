#!/usr/bin/env python3
"""Reproducibly derive per-run phase splits and cell medians from scoreboard CSVs."""

import csv
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[6]
RESULTS = Path(__file__).resolve().parent
INPUTS = [
    (
        "s1-benchy",
        ROOT / "docs/specs/perf-vs-orca/evidence/matched-pair/results/s1-benchy.csv",
    ),
    (
        "s2-base",
        ROOT / "docs/specs/perf-vs-orca/evidence/matched-pair/results/s2-base.csv",
    ),
    (
        "dev174-before",
        ROOT
        / "docs/specs/perf-vs-orca/evidence/dev174-repair/results/dev174-before.csv",
    ),
    (
        "dev174-after",
        ROOT
        / "docs/specs/perf-vs-orca/evidence/dev174-repair/results/dev174-after.csv",
    ),
]
PHASES = ("validation", "prepass", "per_layer", "postpass")
TOOLS = ("pnp-ordinary", "pnp-accelerated")
CSV_FIELDS = [
    "run_id",
    "batch_vintage",
    "vintage_status",
    "fixture",
    "generator",
    "supports",
    "tool",
    "run_index",
    "warmup",
    "wall_seconds",
    "cpu_seconds",
    "cpu_wall_ratio",
    "elapsed_ms",
    "validation_ms",
    "prepass_ms",
    "per_layer_ms",
    "postpass_ms",
    "phases_sum_ms",
    "tail_gap_ms",
    "R_ms",
    "write_and_overhead_ms",
    "degraded",
    "non_fatal_error_count",
    "stderr_path",
    "capture_status",
    "missing_file_marker",
    "phase_complete_count",
    "slice_complete_count",
    "phase_event_counts_json",
    "slice_degraded",
    "slice_fatal_error_count",
    "slice_non_fatal_error_count",
    "exit_code",
]


def as_bool(value):
    if value is None or value == "":
        return None
    if isinstance(value, bool):
        return value
    normalized = str(value).strip().lower()
    if normalized in ("1", "true", "yes"):
        return True
    if normalized in ("0", "false", "no"):
        return False
    return None


def csv_bool(value):
    parsed = as_bool(value)
    return "" if parsed is None else str(parsed).lower()


def as_number(value):
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if math.isfinite(number) else None


def resolve_capture(value):
    if not value:
        return None
    path = Path(value)
    return path if path.is_absolute() else ROOT / path


def vintage_status(vintage, supports):
    if vintage == "s2-base" and supports == "on":
        return "pre-repair (degraded=true expected)"
    if vintage == "s2-base":
        return "supports-off/untainted"
    if vintage == "dev174-before":
        return "pre-repair"
    if vintage == "dev174-after":
        return "post-repair (degraded=false expected)"
    return "matched-pair"


def parse_capture(path):
    phases = {phase: None for phase in PHASES}
    phase_values = defaultdict(list)
    event_counts = Counter()
    slice_values = []
    if path is None:
        return {
            "phases": phases,
            "elapsed_ms": None,
            "event_counts": event_counts,
            "phase_event_counts": Counter(),
            "slice": None,
            "status": "missing_path",
            "missing_marker": "missing_path",
        }
    if not path.exists():
        return {
            "phases": phases,
            "elapsed_ms": None,
            "event_counts": event_counts,
            "phase_event_counts": Counter(),
            "slice": None,
            "status": "missing_file",
            "missing_marker": "missing_file",
        }

    phase_event_counts = Counter()
    with path.open("r", encoding="utf-8-sig", errors="replace") as stream:
        for line in stream:
            try:
                event = json.loads(line)
            except (json.JSONDecodeError, TypeError):
                continue
            if not isinstance(event, dict):
                continue
            event_name = event.get("event")
            if event_name == "phase_complete":
                event_counts[event_name] += 1
                phase = event.get("phase")
                phase_event_counts[str(phase)] += 1
                if phase in phases:
                    value = as_number(event.get("elapsed_ms"))
                    if value is not None:
                        phase_values[phase].append(value)
            elif event_name == "slice_complete":
                event_counts[event_name] += 1
                slice_values.append(event)

    # These captures normally contain one event per phase and one slice event.
    # Retain the last valid event if a log contains duplicates; the printed event
    # counts and JSON count field make any such duplicate visible.
    for phase in PHASES:
        if phase_values[phase]:
            phases[phase] = phase_values[phase][-1]
    slice_event = slice_values[-1] if slice_values else None
    complete = (
        all(value is not None for value in phases.values()) and slice_event is not None
    )
    return {
        "phases": phases,
        "elapsed_ms": as_number(slice_event.get("elapsed_ms")) if slice_event else None,
        "event_counts": event_counts,
        "phase_event_counts": phase_event_counts,
        "slice": slice_event,
        "status": "complete" if complete else "partial",
        "missing_marker": "",
    }


def fmt(value, digits=3):
    if value is None:
        return ""
    return f"{value:.{digits}f}"


def med(rows, field):
    values = [as_number(row.get(field)) for row in rows]
    values = [value for value in values if value is not None]
    return statistics.median(values) if values else None


def all_rows():
    output = []
    expected_counts = {}
    for vintage, csv_path in INPUTS:
        with csv_path.open(newline="", encoding="utf-8-sig") as stream:
            reader = csv.DictReader(stream)
            required = {
                "run_id",
                "fixture",
                "generator",
                "supports",
                "tool",
                "run_index",
                "warmup",
                "wall_seconds",
                "cpu_seconds",
                "cpu_wall_ratio",
                "stderr_path",
                "degraded",
                "non_fatal_error_count",
                "exit_code",
            }
            missing_columns = required.difference(reader.fieldnames or [])
            if missing_columns:
                raise ValueError(
                    f"{csv_path}: missing required columns {sorted(missing_columns)}"
                )
            source_rows = list(reader)

        pnp_rows = [row for row in source_rows if row.get("tool") in TOOLS]
        expected_counts[vintage] = len(pnp_rows)
        for source in pnp_rows:
            capture_path = resolve_capture(source.get("stderr_path"))
            capture = parse_capture(capture_path)
            phases = capture["phases"]
            elapsed = capture["elapsed_ms"]
            phase_sum = (
                sum(phases[phase] for phase in PHASES)
                if all(phases[phase] is not None for phase in PHASES)
                else None
            )
            tail = (
                elapsed - phase_sum
                if elapsed is not None and phase_sum is not None
                else None
            )
            wall = as_number(source.get("wall_seconds"))
            r_ms = (
                wall * 1000.0 - elapsed
                if wall is not None and elapsed is not None
                else None
            )
            slice_event = capture["slice"] or {}
            row = {
                "run_id": source.get("run_id", ""),
                "batch_vintage": vintage,
                "vintage_status": vintage_status(vintage, source.get("supports", "")),
                "fixture": source.get("fixture", ""),
                "generator": source.get("generator", ""),
                "supports": source.get("supports", ""),
                "tool": source.get("tool", ""),
                "run_index": source.get("run_index", ""),
                "warmup": str(bool(as_bool(source.get("warmup")))).lower(),
                "wall_seconds": source.get("wall_seconds", ""),
                "cpu_seconds": source.get("cpu_seconds", ""),
                "cpu_wall_ratio": source.get("cpu_wall_ratio", ""),
                "elapsed_ms": elapsed,
                "validation_ms": phases["validation"],
                "prepass_ms": phases["prepass"],
                "per_layer_ms": phases["per_layer"],
                "postpass_ms": phases["postpass"],
                "phases_sum_ms": phase_sum,
                "tail_gap_ms": tail,
                "R_ms": r_ms,
                "write_and_overhead_ms": r_ms,
                "degraded": csv_bool(source.get("degraded")),
                "non_fatal_error_count": source.get("non_fatal_error_count", ""),
                "stderr_path": source.get("stderr_path", ""),
                "capture_status": capture["status"],
                "missing_file_marker": capture["missing_marker"],
                "phase_complete_count": capture["event_counts"]["phase_complete"],
                "slice_complete_count": capture["event_counts"]["slice_complete"],
                "phase_event_counts_json": json.dumps(
                    dict(sorted(capture["phase_event_counts"].items())),
                    separators=(",", ":"),
                ),
                "slice_degraded": csv_bool(slice_event.get("degraded")),
                "slice_fatal_error_count": slice_event.get("fatal_error_count", ""),
                "slice_non_fatal_error_count": slice_event.get(
                    "non_fatal_error_count", ""
                ),
                "exit_code": source.get("exit_code", ""),
                "_warmup_bool": as_bool(source.get("warmup")),
                "_capture_path": str(capture_path) if capture_path else "",
                "_capture": capture,
            }
            output.append(row)

            phases_text = ",".join(
                f"{phase}={capture['phase_event_counts'].get(phase, 0)}"
                for phase in PHASES
            )
            print(
                "EVENT_COUNTS"
                f" vintage={vintage} run_id={row['run_id']}"
                f" file={capture_path if capture_path else '<empty stderr_path>'}"
                f" phase_complete={capture['event_counts']['phase_complete']}"
                f" slice_complete={capture['event_counts']['slice_complete']}"
                f" phases=[{phases_text}] status={capture['status']}"
            )
    return output, expected_counts


def write_run_csv(rows):
    path = RESULTS / "run-phase-splits.csv"
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=CSV_FIELDS, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            cooked = {}
            for field in CSV_FIELDS:
                value = row.get(field)
                if field.endswith("_ms") or field in ("elapsed_ms",):
                    cooked[field] = fmt(value, 3)
                else:
                    cooked[field] = value
            writer.writerow(cooked)
    return path


def group_rows(rows, keys):
    groups = defaultdict(list)
    for row in rows:
        groups[tuple(row[key] for key in keys)].append(row)
    return groups


def cell_summary(rows):
    return {
        "n": len(rows),
        "n_elapsed": sum(as_number(row.get("elapsed_ms")) is not None for row in rows),
        "n_phases": sum(
            as_number(row.get("phases_sum_ms")) is not None for row in rows
        ),
        "n_R": sum(as_number(row.get("R_ms")) is not None for row in rows),
        "n_tail": sum(as_number(row.get("tail_gap_ms")) is not None for row in rows),
        "wall": med(rows, "wall_seconds"),
        "cpu": med(rows, "cpu_seconds"),
        "R": med(rows, "R_ms"),
        "tail": med(rows, "tail_gap_ms"),
        "phase_sum": med(rows, "phases_sum_ms"),
        "elapsed": med(rows, "elapsed_ms"),
        "wall_min": min(
            (
                as_number(row.get("wall_seconds"))
                for row in rows
                if as_number(row.get("wall_seconds")) is not None
            ),
            default=None,
        ),
        "wall_max": max(
            (
                as_number(row.get("wall_seconds"))
                for row in rows
                if as_number(row.get("wall_seconds")) is not None
            ),
            default=None,
        ),
        "vintages": list(dict.fromkeys(row["batch_vintage"] for row in rows)),
        "degraded": list(dict.fromkeys(row.get("degraded", "") for row in rows)),
    }


def pretty_vintages(vintages, rows):
    labels = []
    for vintage in vintages:
        members = [row for row in rows if row["batch_vintage"] == vintage]
        statuses = list(dict.fromkeys(row["vintage_status"] for row in members))
        labels.append(f"{vintage} ({'; '.join(statuses)})")
    return "; ".join(labels)


def make_markdown(rows):
    measured = [row for row in rows if not row["_warmup_bool"]]
    cell_groups = group_rows(measured, ("fixture", "generator", "supports", "tool"))
    vintage_groups = group_rows(
        measured, ("fixture", "generator", "supports", "tool", "batch_vintage")
    )
    lines = [
        "# T27 serial-floor phase-split cell medians",
        "",
        "Medians below use measured rows only (`warmup=false`). The primary tables pool measured runs by fixture × generator × supports × tool across the supplied vintages, as requested; where repairs create multiple vintages, a second table keeps those vintages separate. `R = wall_seconds × 1000 − elapsed_ms`; `tail_gap = elapsed_ms − sum(validation, prepass, per_layer, postpass)`. Times are in seconds or milliseconds as labeled.",
        "",
        "Supports-off cells are shown separately and are the untainted comparison cells. For supports-on `s2-base`, vintage status is pre-repair (degraded=true expected); `dev174-before` is pre-repair; `dev174-after` is post-repair (degraded=false expected).",
        "",
    ]
    for fixture in ("benchy", "base"):
        lines.extend(
            [
                f"## {fixture}",
                "",
                "| Generator | Supports | Tool | n | Vintage(s) | Median wall (s) | Median CPU (s) | Median R (ms) | Median tail_gap (ms) | Median phases_sum (ms) | Median elapsed (ms) | Wall min–max (s) |",
                "|---|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|",
            ]
        )
        fixture_keys = [key for key in cell_groups if key[0] == fixture]
        fixture_keys.sort(
            key=lambda key: (key[1], key[2] != "off", TOOLS.index(key[3]))
        )
        for key in fixture_keys:
            group = cell_groups[key]
            stats = cell_summary(group)
            lines.append(
                "| "
                + " | ".join(
                    [
                        key[1],
                        key[2],
                        key[3],
                        str(stats["n"]),
                        pretty_vintages(stats["vintages"], group),
                        fmt(stats["wall"], 4),
                        fmt(stats["cpu"], 4),
                        fmt(stats["R"], 1),
                        fmt(stats["tail"], 1),
                        fmt(stats["phase_sum"], 1),
                        fmt(stats["elapsed"], 1),
                        f"{fmt(stats['wall_min'], 4)}–{fmt(stats['wall_max'], 4)}",
                    ]
                )
                + " |"
            )
        lines.append("")

    lines.extend(
        [
            "## Vintage-resolved supports-on results (base)",
            "",
            "This split prevents pre-repair and post-repair runs from being mistaken for one homogeneous sample. `degraded` below is the CSV value observed in the contributing runs.",
            "",
            "| Generator | Tool | Vintage | Repair status | n | degraded value(s) | Median wall (s) | Median R (ms) | Median tail_gap (ms) | Median phases_sum (ms) | Median elapsed (ms) |",
            "|---|---|---|---|---:|---|---:|---:|---:|---:|---:|",
        ]
    )
    vintage_keys = [
        key for key in vintage_groups if key[0] == "base" and key[2] == "on"
    ]
    vintage_order = {
        name: index
        for index, name in enumerate(("s2-base", "dev174-before", "dev174-after"))
    }
    vintage_keys.sort(
        key=lambda key: (key[1], TOOLS.index(key[3]), vintage_order.get(key[4], 99))
    )
    for key in vintage_keys:
        group = vintage_groups[key]
        stats = cell_summary(group)
        lines.append(
            "| "
            + " | ".join(
                [
                    key[1],
                    key[3],
                    key[4],
                    group[0]["vintage_status"],
                    str(stats["n"]),
                    ", ".join(stats["degraded"]),
                    fmt(stats["wall"], 4),
                    fmt(stats["R"], 1),
                    fmt(stats["tail"], 1),
                    fmt(stats["phase_sum"], 1),
                    fmt(stats["elapsed"], 1),
                ]
            )
            + " |"
        )
    lines.append("")

    lines.extend(["## Observations", ""])
    for fixture in ("benchy", "base"):
        for tool in TOOLS:
            subset = [
                row
                for row in measured
                if row["fixture"] == fixture and row["tool"] == tool
            ]
            r_values = [as_number(row.get("R_ms")) for row in subset]
            r_values = [value for value in r_values if value is not None]
            if r_values:
                lines.append(
                    f"- Typical `R` for **{fixture} / {tool}**: median {fmt(statistics.median(r_values), 1)} ms, observed range {fmt(min(r_values), 1)}–{fmt(max(r_values), 1)} ms across measured rows."
                )

    for fixture in ("benchy", "base"):
        details = []
        fixture_cells = [key for key in cell_groups if key[0] == fixture]
        fixture_cells.sort(
            key=lambda key: (key[1], key[2] != "off", TOOLS.index(key[3]))
        )
        for key in fixture_cells:
            group = cell_groups[key]
            ratios = []
            gaps = []
            for row in group:
                elapsed = as_number(row.get("elapsed_ms"))
                gap = as_number(row.get("tail_gap_ms"))
                if elapsed not in (None, 0) and gap is not None:
                    ratios.append(gap / elapsed * 100.0)
                    gaps.append(gap)
            if ratios:
                label = f"{key[1]}/{key[2]}/{key[3]}"
                threshold = (
                    "material" if statistics.median(ratios) > 5.0 else "not material"
                )
                details.append(
                    f"{label}: {threshold}, median share {fmt(statistics.median(ratios), 2)}% (range {fmt(min(ratios), 2)}–{fmt(max(ratios), 2)}%); tail range {fmt(min(gaps), 1)}–{fmt(max(gaps), 1)} ms"
                )
        if details:
            lines.append(
                f"- **Tail-gap materiality, {fixture}** (material means median per-run `tail_gap / elapsed_ms` > 5%; per-cell spread shown): "
                + "; ".join(details)
                + "."
            )

    impossible = []
    for row in rows:
        wall_ms = as_number(row.get("wall_seconds"))
        elapsed = as_number(row.get("elapsed_ms"))
        if wall_ms is not None and elapsed is not None and wall_ms * 1000.0 < elapsed:
            impossible.append((row, wall_ms * 1000.0 - elapsed))
    if impossible:
        lines.append(
            "- **Impossible ordering (wall < elapsed):** "
            + "; ".join(
                f"{row['batch_vintage']}/{row['run_id']} wall={fmt(as_number(row['wall_seconds']) * 1000.0, 1)} ms, elapsed={fmt(as_number(row['elapsed_ms']), 1)} ms (R={fmt(delta, 1)} ms)"
                for row, delta in impossible
            )
            + "."
        )
    else:
        lines.append(
            "- **Impossible ordering (wall < elapsed):** none among captures with both values present."
        )

    lines.append(
        "- **dev174 R comparison against `s2-base`:** values below compare each dev174 measured run with the same-cell `s2-base` measured-run median and range."
    )
    for key in (
        ("base", "classic", "on", "pnp-ordinary"),
        ("base", "classic", "on", "pnp-accelerated"),
        ("base", "arachne", "on", "pnp-ordinary"),
        ("base", "arachne", "on", "pnp-accelerated"),
    ):
        vintage_key = ("base", key[1], "on", key[3], "s2-base")
        baseline = vintage_groups.get(vintage_key, [])
        baseline_rs = [as_number(row.get("R_ms")) for row in baseline]
        baseline_rs = [value for value in baseline_rs if value is not None]
        for vintage in ("dev174-before", "dev174-after"):
            dev_rows = vintage_groups.get(("base", key[1], "on", key[3], vintage), [])
            for dev_row in dev_rows:
                if not baseline_rs or as_number(dev_row.get("R_ms")) is None:
                    continue
                lines.append(
                    f"  - `{key[1]}/on/{key[3]}` {vintage}: R={fmt(as_number(dev_row['R_ms']), 1)} ms vs `s2-base` median {fmt(statistics.median(baseline_rs), 1)} ms (range {fmt(min(baseline_rs), 1)}–{fmt(max(baseline_rs), 1)} ms)."
                )

    missing = [row for row in rows if row["missing_file_marker"]]
    partial = [row for row in rows if row["capture_status"] == "partial"]
    lines.append(
        f"- Capture coverage: {len(rows) - len(missing)} existing files, {len(missing)} missing-file/path markers, {len(partial)} partial captures; every PNP scoreboard row remains in the run CSV."
    )
    lines.extend(
        [
            "",
            "## Input schema and calculation notes",
            "",
            "All four inputs used the same scoreboard header: `run_id`, `fixture`, `generator`, `supports`, `tool`, `run_index`, `warmup`, `wall_seconds`, `cpu_seconds`, `cpu_wall_ratio`, `stderr_path`, `degraded`, `non_fatal_error_count`, and `exit_code` were present. Non-JSON stderr lines and JSON objects without `event=phase_complete` or `event=slice_complete` were ignored. Duplicate recognized events, if present, are counted and the last valid event is used for that field.",
            "",
        ]
    )
    return "\n".join(lines)


def hand_check(rows):
    selected = next(
        row
        for row in rows
        if row["batch_vintage"] == "s1-benchy"
        and row["run_id"] == "benchy-classic-off-pnp-ordinary-m1"
    )
    capture = selected["_capture"]
    raw_phases = [capture["phases"][phase] for phase in PHASES]
    phase_sum = sum(raw_phases)
    elapsed = capture["elapsed_ms"]
    wall_ms = as_number(selected["wall_seconds"]) * 1000.0
    print(
        "HAND_CHECK"
        f" file={selected['_capture_path']}"
        f" run_id={selected['run_id']}"
        f" phases_ms={dict(zip(PHASES, raw_phases))}"
        f" sum={phase_sum:.3f} ms"
        f" elapsed={elapsed:.3f} ms"
        f" tail={elapsed - phase_sum:.3f} ms"
        f" wall={wall_ms:.3f} ms"
        f" R={wall_ms - elapsed:.3f} ms"
        f" output_tail={selected['tail_gap_ms']:.3f} ms"
        f" output_R={selected['R_ms']:.3f} ms"
    )
    assert math.isclose(phase_sum, selected["phases_sum_ms"], abs_tol=1e-9)
    assert math.isclose(elapsed - phase_sum, selected["tail_gap_ms"], abs_tol=1e-9)
    assert math.isclose(wall_ms - elapsed, selected["R_ms"], abs_tol=1e-9)


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows, expected_counts = all_rows()
    output_path = write_run_csv(rows)
    markdown_path = RESULTS / "CELL-MEDIANS.md"
    markdown_path.write_text(make_markdown(rows), encoding="utf-8")
    observed_counts = Counter(row["batch_vintage"] for row in rows)
    missing = [row for row in rows if row["missing_file_marker"]]
    print("ROW_COVERAGE")
    for vintage, _ in INPUTS:
        print(
            f"  {vintage}: scoreboard PNP rows={expected_counts[vintage]}, output rows={observed_counts[vintage]}"
        )
        if observed_counts[vintage] != expected_counts[vintage]:
            raise AssertionError(f"PNP row count mismatch for {vintage}")
    print(f"TOTAL PNP rows={sum(expected_counts.values())}, output rows={len(rows)}")
    print(f"MISSING_CAPTURE_MARKERS={len(missing)}")
    hand_check(rows)
    print(f"WROTE {output_path}")
    print(f"WROTE {markdown_path}")


if __name__ == "__main__":
    main()
