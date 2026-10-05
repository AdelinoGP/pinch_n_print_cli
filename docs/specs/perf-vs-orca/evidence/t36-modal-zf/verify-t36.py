"""Independently compare modal G-code semantics and verify ticket 36 evidence.

Default verification is read-only and needs only evidence.json and protocol.json.
--reduce ROOT reads optional target/t36-ab captures; it never launches slices.
"""

import argparse
from collections import defaultdict
import csv
import hashlib
import json
import math
from pathlib import Path
import re
from statistics import median

HERE = Path(__file__).resolve().parent
ARMS = ("baseline", "candidate")
METRICS = ("wall_seconds", "cpu_seconds")
# Job identity: the licensed 3dbenchy fixture hashes are pinned rather than
# re-derived, because the fixture is gitignored and must not be committed.
JOB_MODEL_SHA256 = "6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def axis_words(line):
    """Return only parseable, finite axis words before an inline comment."""
    words = []
    for token in line.split(";", 1)[0].split()[1:]:
        if token[0].upper() not in "XYZFE":
            continue
        try:
            value = float(token[1:])
        except ValueError:
            continue
        if math.isfinite(value):
            words.append((token[0].upper(), value, token[1:]))
    return words


def normalize_text(text, byte_count):
    position = dict(x=None, y=None, z=None, f=None, e=0.0)
    abs_xyz = abs_e = True
    records = []
    counts = dict(
        bytes=byte_count,
        line_count=0,
        move_line_count=0,
        z_token_count=0,
        f_token_count=0,
        e_token_count=0,
    )
    emitted_z = set()
    for original in text.splitlines():
        line = original.strip()
        if not line:
            continue
        counts["line_count"] += 1
        if line.startswith(";"):
            records.append({"kind": "comment", "text": line})
            continue
        cmd = line.split()[0].upper()
        if cmd in ("G0", "G1"):
            counts["move_line_count"] += 1
            for axis, value, spelling in axis_words(line):
                key = axis.lower()
                if axis in "ZFE":
                    counts[key + "_token_count"] += 1
                if axis == "Z":
                    emitted_z.add(spelling)
                if axis in "XYZ":
                    position[key] = (
                        value
                        if abs_xyz
                        else (None if position[key] is None else position[key] + value)
                    )
                elif axis == "E":
                    position[key] = value if abs_e else position[key] + value
                else:
                    position[key] = value
            records.append(
                {
                    "kind": "move",
                    "cmd": cmd,
                    "abs_e": abs_e,
                    **{
                        key: None if value is None else round(value, 6)
                        for key, value in position.items()
                    },
                }
            )
        else:
            if cmd == "G92":
                for axis, value, _ in axis_words(line):
                    if axis == "E":
                        position["e"] = value
            elif cmd in ("G90", "G91"):
                abs_xyz = cmd == "G90"
            elif cmd in ("M82", "M83"):
                abs_e = cmd == "M82"
            records.append({"kind": "raw", "text": line})
    counts["distinct_emitted_z_count"] = len(emitted_z)
    counts["stream_sha256"] = hashlib.sha256(
        json.dumps(records, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return records, counts


def normalize(path):
    """Interpret modal state independently of the Rust serializer."""
    raw = path.read_bytes()
    return normalize_text(raw.decode("utf-8-sig"), len(raw))


def compare_normalized(baseline, candidate):
    left, left_counts = baseline
    right, right_counts = candidate
    first_diff = None
    for index, (a, b) in enumerate(zip(left, right)):
        if a != b:
            first_diff = {"index": index, "baseline": a, "candidate": b}
            break
    if first_diff is None and len(left) != len(right):
        first_diff = {
            "index": min(len(left), len(right)),
            "baseline_length": len(left),
            "candidate_length": len(right),
        }
    return {
        "equivalent": left == right,
        "baseline": left_counts,
        "candidate": right_counts,
        "first_diff": first_diff,
    }


def compare_text(left, right):
    return compare_normalized(
        normalize_text(left, len(left.encode("utf-8"))),
        normalize_text(right, len(right.encode("utf-8"))),
    )


def mutate(text):
    """Sabotage the first Z move token, falling back to the first F token."""
    lines = text.splitlines(keepends=True)
    for axis, increment in (("Z", 1.0), ("F", 1000.0)):
        for index, line in enumerate(lines):
            words = line.strip().split()
            if not words or words[0].upper() not in ("G0", "G1"):
                continue
            for key, value, spelling in axis_words(line):
                if key == axis:
                    # Locate a whole token, not a matching substring in a comment.
                    pattern = r"(?<!\S)" + axis + re.escape(spelling) + r"(?=\s|;|$)"
                    lines[index] = re.sub(
                        pattern,
                        axis + str(value + increment),
                        line,
                        count=1,
                        flags=re.IGNORECASE,
                    )
                    require(lines[index] != line, "mutation did not change token")
                    return "".join(lines)
    raise RuntimeError("candidate has no parseable move-line Z or F to falsify")


def self_test():
    baseline = """;LAYER:0
;HEIGHT:0.2
G90
M82
G1 X0 Y0 Z0.2 E1 F1200
G1 X1 Y1 Z0.2 E2 F1200
G0 Z0.6 F1200
G1 E1.5 F1200
G1 E1 F900
G92 E0
M83
G1 E-0.8 F900
G1 E0.8 F900
"""
    modal = """;LAYER:0
;HEIGHT:0.2
G90
M82
G1 X0 Y0 Z0.2 E1 F1200
G1 X1 Y1 E2
G0 Z0.6
G1 E1.5
G1 E1 F900
G92 E0
M83
G1 E-0.8
G1 E0.8
"""
    require(compare_text(baseline, modal)["equivalent"], "modal omission rejected")
    sabotages = {
        "dropped changing Z": modal.replace("G0 Z0.6", "G0"),
        "changed Z": modal.replace("Z0.6", "Z1.6"),
        "changed F": modal.replace("F900", "F1900", 1),
        "dropped M83": modal.replace("M83\n", ""),
        "automatic mutation": mutate(modal),
    }
    for name, text in sabotages.items():
        require(not compare_text(baseline, text)["equivalent"], "undetected " + name)
    # Direct state checks make mode handling independently falsifiable, even
    # though preserving raw commands already detects a missing mode line.
    records, counts = normalize_text(modal, len(modal.encode("utf-8")))
    moves = [record for record in records if record["kind"] == "move"]
    require(
        moves[-2]["e"] == -0.8 and moves[-1]["e"] == 0.0,
        "M83 accumulation or G92 reset",
    )
    extra = "G1 X10 Y20 Z1 E5 F100\nG91\nM83\nG1 X2 Y-3 Z0.5 E-1\nG92 E7\nG1 E1\nG90\nM82\nG1 X4 E2\n"
    records, _ = normalize_text(extra, len(extra))
    require(
        records[3]["x"] == 12
        and records[3]["y"] == 17
        and records[3]["z"] == 1.5
        and records[3]["e"] == 4,
        "relative coordinates/extrusion",
    )
    require(
        records[5]["e"] == 8 and records[-1]["e"] == 2 and records[-1]["x"] == 4,
        "G92 or absolute mode restoration",
    )
    require(
        counts["z_token_count"] == 2 and counts["f_token_count"] == 2, "token counting"
    )
    print("SELF-TEST PASS")


def batch_key(fixture, cell, mode):
    return json.dumps([fixture, cell, mode], separators=(",", ":"))


def batches(rows):
    result = defaultdict(list)
    for row in rows:
        key = batch_key(row["fixture"], row["cell"], row["mode"])
        require(row["source"] == key, "row source mismatch")
        result[key].append(row)
    return dict(sorted(result.items()))


def timing_summary(rows, protocol):
    grouped = batches(rows)
    require(grouped, "no measurement batches")
    modes = {mode: {} for mode in protocol["modes"]}
    for key, batch in grouped.items():
        for row in batch:
            require(
                row["mode"] in modes
                and row["cell"] in protocol["cells"]
                and row["arm"] in ARMS
                and row["warmup"] in (0, 1),
                "unexpected mode/cell/arm/warmup",
            )
            require(
                row["threads"] == protocol["threads"]
                and row["completion_status"] == "ok"
                and not row["degraded"]
                and row["fatal_error_count"]
                == row["non_fatal_error_count"]
                == row["exit_code"]
                == 0,
                "unsuccessful capture: " + row["run_id"],
            )
            require(
                all(
                    math.isfinite(row[k]) and row[k] > 0
                    for k in (*METRICS, "cpu_wall_ratio")
                ),
                "invalid timing",
            )
            # The driver rounds wall/cpu/ratio to 4 decimals before writing the
            # CSV, so a ratio recomputed from the rounded wall/cpu can differ
            # from the stored rounded ratio by up to ~1e-4 (measured worst:
            # 5.2e-5). 2e-4 admits the rounding without admitting a wrong ratio.
            require(
                math.isclose(
                    row["cpu_wall_ratio"],
                    row["cpu_seconds"] / row["wall_seconds"],
                    rel_tol=0,
                    abs_tol=2e-4,
                ),
                "ratio arithmetic",
            )
        measured = [r for r in batch if r["warmup"] == 0]
        for arm in ARMS:
            samples = [r for r in measured if r["arm"] == arm]
            require(
                len(samples) == protocol["measured_pairs_per_mode"],
                "measurement population: " + key,
            )
            require(
                sum(r["warmup"] == 1 and r["arm"] == arm for r in batch)
                == protocol["excluded_warmups_per_arm"],
                "warmup population: " + key,
            )
            require(
                len({r["sha256_of_gcode"] for r in samples}) == 1,
                "non-deterministic arm: " + key + "/" + arm,
            )
        paired_all = defaultdict(dict)
        for row in measured:
            pair = paired_all[row["run_index"]]
            require(row["arm"] not in pair, "duplicate pair member: " + key)
            pair[row["arm"]] = row
        require(
            all(set(pair) == set(ARMS) for pair in paired_all.values()),
            "incomplete pair: " + key,
        )
        cutoffs = {
            arm: protocol["starvation"]["cutoff_fraction_of_best_cpu_wall_ratio"]
            * max(r["cpu_wall_ratio"] for r in measured if r["arm"] == arm)
            for arm in ARMS
        }
        excluded = sorted(
            {
                r["run_index"]
                for r in measured
                if r["cpu_wall_ratio"] < cutoffs[r["arm"]]
            }
        )
        retained = [r for r in measured if r["run_index"] not in excluded]
        stats = {}
        for arm in ARMS:
            samples = [r for r in retained if r["arm"] == arm]
            stats[arm] = {
                metric: median(r[metric] for r in samples) if samples else None
                for metric in METRICS
            }
            ratios = [r["cpu_wall_ratio"] for r in samples]
            stats[arm]["cpu_wall_range"] = [min(ratios), max(ratios)] if ratios else []
        paired = [
            paired_all[index] for index in sorted(paired_all) if index not in excluded
        ]
        deltas = {
            metric: [p["candidate"][metric] - p["baseline"][metric] for p in paired]
            for metric in METRICS
        }
        passed = (
            bool(paired)
            and len(paired) >= protocol["starvation"]["minimum_retained_pairs_per_mode"]
            and all(
                stats["candidate"][metric] < stats["baseline"][metric]
                and median(deltas[metric]) < 0
                for metric in METRICS
            )
        )
        result = {
            "retained_pairs": len(paired),
            "excluded_pairs": excluded,
            "starvation_cutoffs": cutoffs,
            "medians": stats,
            "paired_median_deltas": {
                metric: median(values) if values else None
                for metric, values in deltas.items()
            },
            "pairs_favoring_candidate": {
                metric: sum(v < 0 for v in values) for metric, values in deltas.items()
            },
            "strict_range_separation": {
                metric: bool(paired)
                and max(p["candidate"][metric] for p in paired)
                < min(p["baseline"][metric] for p in paired)
                for metric in METRICS
            },
            "gate_passed": passed,
        }
        for name, metric in (("wall", "wall_seconds"), ("cpu", "cpu_seconds")):
            result[name + "_reduction_percent"] = (
                100 * (1 - stats["candidate"][metric] / stats["baseline"][metric])
                if paired
                else None
            )
        modes[batch[0]["mode"]][key] = result
    fixtures = {r["fixture"] for r in rows}
    require(
        set(grouped)
        == {
            batch_key(fixture, cell, mode)
            for fixture in fixtures
            for cell in protocol["cells"]
            for mode in protocol["modes"]
        },
        "missing protocol cell/mode batch",
    )
    return {
        "recommendation": "KEEP"
        if all(
            entry["gate_passed"] for mode in modes.values() for entry in mode.values()
        )
        else "inconclusive/DROP",
        "automatic_commit": False,
        "scope": protocol["job"],
        "modes": modes,
    }


def derive_inputs(require_model):
    """Hash the job inputs; the model is a licensed gitignored fixture.

    `require_model=True` (reduce, on the measurement machine) fails if the
    model is absent. `require_model=False` (tracked-evidence verification)
    substitutes the pinned job-identity hash when the file is not checked out;
    when it is present, its real hash is used and must match the pin.
    """
    root = HERE.parents[4]
    model = root / "tmp" / "3dbenchy.stl"
    if require_model:
        require(model.is_file(), f"missing model: {model}")
    model_sha = digest(model) if model.is_file() else JOB_MODEL_SHA256
    require(model_sha == JOB_MODEL_SHA256, "job identity drift: " + model_sha)
    configs = {
        cell: root
        / "docs"
        / "specs"
        / "perf-vs-orca"
        / "evidence"
        / "matched-pair"
        / "configs"
        / f"pnp-{cell.split('-')[0]}-supports-{cell.split('-')[1]}.json"
        for cell in ("classic-off", "arachne-off")
    }
    for path in configs.values():
        require(path.is_file(), f"missing config: {path}")
    return {
        "model": "tmp/3dbenchy.stl",
        "model_sha256": model_sha,
        "configs": {
            cell: {
                "path": f"docs/specs/perf-vs-orca/evidence/matched-pair/configs/{path.name}",
                "sha256": digest(path),
            }
            for cell, path in configs.items()
        },
    }


def reduce(raw):
    protocol = read_json(HERE / "protocol.json")
    rows = []
    outputs = {}
    integer_keys = (
        "run_index",
        "warmup",
        "threads",
        "peak_workingset_bytes",
        "gcode_bytes",
        "fatal_error_count",
        "non_fatal_error_count",
        "exit_code",
    )
    paths = sorted((raw / "runs").glob("*/ab-rows.csv"))
    require(paths, "no runs/*/ab-rows.csv captures")
    for path in paths:
        with path.open(encoding="utf-8-sig", newline="") as stream:
            for row in csv.DictReader(stream):
                for key in integer_keys:
                    row[key] = int(row[key])
                for key in (*METRICS, "cpu_wall_ratio"):
                    row[key] = float(row[key])
                require(
                    row["degraded"].lower() in ("true", "false", "0", "1"),
                    "invalid degraded boolean",
                )
                row["degraded"] = row["degraded"].lower() in ("true", "1")
                row["source"] = batch_key(row["fixture"], row["cell"], row["mode"])
                run_id = row["run_id"]
                require(
                    Path(run_id).name == run_id
                    and "/" not in run_id
                    and "\\" not in run_id
                    and run_id not in (".", ".."),
                    "invalid run_id",
                )
                output = path.parent / (run_id + ".gcode")
                require(
                    digest(output) == row["sha256_of_gcode"]
                    and output.stat().st_size == row["gcode_bytes"],
                    "CSV/output mismatch: " + run_id,
                )
                stderr = path.parent / (run_id + ".stderr.jsonl")
                stderr_text = stderr.read_text(encoding="utf-8-sig")
                events = [
                    json.loads(line)
                    for line in stderr_text.splitlines()
                    if line.lstrip().startswith("{")
                ]
                completions = [e for e in events if e.get("event") == "slice_complete"]
                require(len(completions) == 1, "completion cardinality: " + run_id)
                require(
                    all(
                        completions[0][event_key] == row[row_key]
                        for event_key, row_key in (
                            ("status", "completion_status"),
                            ("degraded", "degraded"),
                            ("fatal_error_count", "fatal_error_count"),
                            ("non_fatal_error_count", "non_fatal_error_count"),
                        )
                    ),
                    "CSV/completion mismatch: " + run_id,
                )
                # Protocol disclosure: existing open-loop warnings must remain
                # visible; they are reported on stderr, not in the CSV.
                row["open_loop_warnings"] = sum(
                    "is not closed" in line for line in stderr_text.splitlines()
                )
                require(
                    not any("PERF-T" in line for line in stderr_text.splitlines()),
                    "probe leakage: " + run_id,
                )
                outputs[id(row)] = output
                rows.append(row)
    summary = timing_summary(rows, protocol)
    oracle = {}
    for key, batch in batches(rows).items():
        pair = [
            next(r for r in batch if r["arm"] == arm and r["warmup"] == 0)
            for arm in ARMS
        ]
        oracle[key] = compare_normalized(*(normalize(outputs[id(r)]) for r in pair))
        require(oracle[key]["equivalent"], "oracle mismatch: " + key)
    # One-variable isolation: within a mode, only the executable may differ
    # between arms. Accelerated arms additionally carry their own module tree.
    snapshots = {
        p.relative_to(raw / "snapshots").as_posix(): digest(p)
        for p in sorted((raw / "snapshots").rglob("*"))
        if p.is_file()
    }
    for mode in protocol["modes"]:
        pair = {arm: {} for arm in ARMS}
        for arm in ARMS:
            prefix = f"{mode}-{arm}/"
            for name, value in snapshots.items():
                if name.startswith(prefix):
                    pair[arm][name[len(prefix) :]] = value
        require(pair["baseline"], f"missing snapshot inventory: {mode}")
        common = set(pair["baseline"]) & set(pair["candidate"])
        changed = sorted(
            name for name in common if pair["baseline"][name] != pair["candidate"][name]
        )
        require(
            changed == ["pnp_cli.exe"],
            f"{mode}: only pnp_cli.exe may differ between arms, got {changed}",
        )
        require(
            set(pair["baseline"]) == set(pair["candidate"]),
            f"{mode}: snapshot file sets differ",
        )
    data = {
        "protocol": protocol,
        "protocol_sha256": digest(HERE / "protocol.json"),
        "snapshots": snapshots,
        "inputs": derive_inputs(require_model=True),
        "oracle": oracle,
        "rows": rows,
        "summary": summary,
        "raw_root_optional": str(raw),
    }
    (HERE / "evidence.json").write_text(
        json.dumps(data, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))


def verify():
    data = read_json(HERE / "evidence.json")
    protocol = read_json(HERE / "protocol.json")
    require(
        data["protocol"] == protocol
        and data["protocol_sha256"] == digest(HERE / "protocol.json"),
        "protocol identity",
    )
    require(data["inputs"] == derive_inputs(require_model=False), "input identity")
    summary = timing_summary(data["rows"], protocol)
    require(summary == data["summary"], "saved summary drift")
    require(set(data["oracle"]) == set(batches(data["rows"])), "oracle batch coverage")
    for key, entry in data["oracle"].items():
        require(
            entry["equivalent"] is True and entry["first_diff"] is None,
            "oracle mismatch: " + key,
        )
        require(
            entry["baseline"]["stream_sha256"] == entry["candidate"]["stream_sha256"],
            "semantic stream identity: " + key,
        )
        for arm in ARMS:
            require(
                all(
                    r["gcode_bytes"] == entry[arm]["bytes"]
                    for r in data["rows"]
                    if r["source"] == key and r["arm"] == arm and r["warmup"] == 0
                ),
                "oracle/row byte count: " + key,
            )
    print(json.dumps(data["summary"], indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--self-test", action="store_true", help="run sabotage fixtures")
    modes.add_argument(
        "--oracle",
        nargs=2,
        type=Path,
        metavar=("BASELINE", "CANDIDATE"),
        help="compare physical modal move streams",
    )
    modes.add_argument(
        "--reduce", type=Path, metavar="RAW_ROOT", help="reduce raw captures"
    )
    parser.add_argument(
        "--falsify", action="store_true", help="mutate candidate in oracle mode"
    )
    args = parser.parse_args()
    if args.falsify and not args.oracle:
        parser.error("--falsify requires --oracle")
    if args.self_test:
        self_test()
    elif args.oracle:
        baseline, candidate = args.oracle
        result = compare_normalized(normalize(baseline), normalize(candidate))
        print(json.dumps(result, indent=2))
        falsifiable = True
        if args.falsify:
            text = mutate(candidate.read_text(encoding="utf-8-sig"))
            falsifiable = not compare_normalized(
                normalize(baseline), normalize_text(text, len(text.encode("utf-8")))
            )["equivalent"]
            print(json.dumps({"falsifiable": falsifiable}))
        require(
            result["equivalent"] and falsifiable,
            "oracle equivalence/falsifiability failed",
        )
    elif args.reduce:
        reduce(args.reduce.resolve())
    else:
        verify()


if __name__ == "__main__":
    main()
