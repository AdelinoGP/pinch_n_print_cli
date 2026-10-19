"""Reduce optional raw captures, or verify the tracked evidence alone.

Default verification needs no executables, licensed models, or raw tree.
--reduce ROOT reads this experiment's raw target/t44-ab tree and replaces only
the derived evidence.json. It never launches slices or reruns measurements.
"""

import argparse
from collections import Counter
import csv
import hashlib
import json
import math
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def inventory(root):
    return {
        p.relative_to(root).as_posix(): digest(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def disclosure(output, stderr):
    lines = stderr.read_text(encoding="utf-8-sig").splitlines()
    events = [json.loads(line) for line in lines if line.lstrip().startswith("{")]
    completions = [e for e in events if e["event"] == "slice_complete"]
    require(len(completions) == 1, f"completion cardinality: {stderr}")
    completion = completions[0]
    return {
        "sha256": digest(output),
        "bytes": output.stat().st_size,
        "type_counts": dict(
            Counter(
                line[6:]
                for line in output.read_text().splitlines()
                if line.startswith(";TYPE:")
            )
        ),
        "completion": {
            key: completion[key]
            for key in (
                "status",
                "degraded",
                "fatal_error_count",
                "non_fatal_error_count",
            )
        },
        "event_counts": dict(Counter(e["event"] for e in events)),
        "open_loop_warnings": sum("is not closed" in line for line in lines),
        "shadow_warnings": sum("shadows integrated" in line for line in lines),
        "probe_lines": sum("PERF-T" in line for line in lines),
    }


def reduce(raw):
    protocol = read_json(HERE / "protocol.json")
    metadata = read_json(raw / "snapshots/capture-metadata.json")
    require(metadata["protocol"] == protocol, "captured protocol identity")
    arms = read_json(raw / "snapshots/arm-inventory.json")
    for mode, pair in arms.items():
        for arm in ("baseline", "candidate"):
            require(
                inventory(raw / f"snapshots/{mode}-{arm}") == pair[arm],
                f"snapshot identity changed after timing: {mode}/{arm}",
            )
    rows = []
    for phase in ("proof", "measure"):
        for mode in protocol["modes"]:
            run_dir = raw / phase / mode
            with (run_dir / "rows.csv").open(
                encoding="utf-8-sig", newline=""
            ) as stream:
                for row in csv.DictReader(stream):
                    for key in (
                        "pair",
                        "warmup",
                        "threads",
                        "bytes",
                        "fatal",
                        "non_fatal",
                        "open_loop_warnings",
                        "exit_code",
                    ):
                        row[key] = int(row[key])
                    for key in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
                        row[key] = float(row[key])
                    row["degraded"] = row["degraded"].lower() == "true"
                    row["type_counts"] = json.loads(row["type_counts"])
                    run_id = row["run_id"]
                    actual = disclosure(
                        run_dir / f"{run_id}.gcode", run_dir / f"{run_id}.stderr.jsonl"
                    )
                    for key in ("sha256", "bytes", "type_counts", "open_loop_warnings"):
                        require(
                            actual[key] == row[key],
                            f"CSV/output mismatch: {run_id}/{key}",
                        )
                    require(
                        actual["completion"]
                        == {
                            "status": row["status"],
                            "degraded": row["degraded"],
                            "fatal_error_count": row["fatal"],
                            "non_fatal_error_count": row["non_fatal"],
                        },
                        f"CSV/completion mismatch: {run_id}",
                    )
                    row["event_counts"] = actual["event_counts"]
                    row["shadow_warnings"] = actual["shadow_warnings"]
                    row["probe_lines"] = actual["probe_lines"]
                    rows.append(row)
    recon = raw / "recon"
    controls = {}
    for label in ("frozen", "head"):
        stem = f"{label}-exact-flags"
        controls[label] = read_json(recon / f"{stem}.result.json")
        controls[label]["disclosure"] = disclosure(
            recon / f"{stem}.gcode", recon / f"{stem}.jsonl"
        )
    data = {
        **metadata,
        "raw_root_optional": str(raw),
        "arms": arms,
        "controls": controls,
        "rows": rows,
    }
    (HERE / "evidence.json").write_text(
        json.dumps(data, indent=2) + "\n", encoding="utf-8"
    )


def verify():
    data = read_json(HERE / "evidence.json")
    protocol = read_json(HERE / "protocol.json")
    require(
        data["protocol"] == protocol
        and data["protocol_sha256"] == digest(HERE / "protocol.json"),
        "protocol identity",
    )
    require(
        data["config_sha256"] == digest(HERE / "benchy-arachne.json"), "config identity"
    )
    fixture = (
        ROOT
        / "crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json"
    )
    require(
        data["regression_reference_sha256"] == digest(fixture),
        "regression-reference identity",
    )
    require(
        data["model_sha256"]
        == "6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7",
        "job identity",
    )
    controls = data["controls"]
    require(
        controls["frozen"]["output_sha256"] == controls["frozen"]["reference_sha256"],
        "frozen control",
    )
    require(
        controls["head"]["output_sha256"] != controls["head"]["reference_sha256"],
        "disclosed HEAD drift",
    )
    for control in controls.values():
        require(
            control["output_sha256"] == control["disclosure"]["sha256"],
            "control output identity",
        )
        require(
            control["disclosure"]["completion"]
            == {
                "status": "ok",
                "degraded": False,
                "fatal_error_count": 0,
                "non_fatal_error_count": 0,
            },
            "control completion",
        )
        require(
            control["exit_code"] == 0
            and "--no-default-module-paths" in control["argv"]
            and "--no-integrated-modules" in control["argv"],
            "control isolation",
        )
        require(control["disclosure"]["shadow_warnings"] == 0, "control shadowing")
    require(
        controls["head"]["disclosure"]["type_counts"]
        != controls["frozen"]["disclosure"]["type_counts"],
        "HEAD drift includes TYPE counts, not merely comments",
    )
    rows = data["rows"]
    require(len(rows) == 32, "four proofs plus twenty-eight warmup/measured slices")
    summary = {}
    for mode in protocol["modes"]:
        pair = data["arms"][mode]
        require(
            pair["baseline"].keys() == pair["candidate"].keys(), "snapshot file set"
        )
        changes = [
            key
            for key in pair["baseline"]
            if pair["baseline"][key] != pair["candidate"][key]
        ]
        require(
            changes == pair["changed"] == [protocol["only_arm_difference"]],
            "one-variable arm identity",
        )
        proof = [r for r in rows if r["mode"] == mode and r["phase"] == "proof"]
        require(
            len(proof) == 2 and {r["arm"] for r in proof} == {"baseline", "candidate"},
            "proof pair",
        )
        require(
            proof[0]["sha256"]
            == proof[1]["sha256"]
            == controls["head"]["output_sha256"],
            "proof exactness",
        )
        disclosure_keys = ("bytes", "type_counts", "open_loop_warnings")
        measured_all = [
            r for r in rows if r["mode"] == mode and r["phase"] == "measure"
        ]
        require(len(measured_all) == 14, "measurement population")
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in measured_all[:2]]
            == [("baseline", 0, 1), ("candidate", 0, 1)],
            "one warmup per arm",
        )
        order = [
            (arm, index, 0)
            for index in range(1, 7)
            for arm in (
                ("baseline", "candidate") if index % 2 else ("candidate", "baseline")
            )
        ]
        measured = measured_all[2:]
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in measured] == order,
            "interleaving",
        )
        for row in proof + measured_all:
            require(
                row["sha256"] == proof[0]["sha256"]
                and all(row[k] == proof[0][k] for k in disclosure_keys),
                "exact output/disclosure",
            )
            require(
                row["threads"] == 12
                and row["status"] == "ok"
                and not row["degraded"]
                and row["fatal"] == row["non_fatal"] == row["exit_code"] == 0
                and row["generator"] == "arachne",
                "completion/generator",
            )
            require(
                row["shadow_warnings"] == row["probe_lines"] == 0
                and all(
                    row["event_counts"].get(event, 0) == 0
                    for event in (
                        "module_complete",
                        "stage_complete",
                        "profile_summary",
                    )
                ),
                "probe-free/isolation",
            )
            require(
                math.isclose(
                    row["cpu_wall_ratio"],
                    row["cpu_seconds"] / row["wall_seconds"],
                    rel_tol=1e-12,
                ),
                "ratio arithmetic",
            )
        cutoffs = {
            arm: protocol["starvation"]["cutoff_fraction_of_best_cpu_wall_ratio"]
            * max(r["cpu_wall_ratio"] for r in measured if r["arm"] == arm)
            for arm in ("baseline", "candidate")
        }
        excluded = sorted(
            {r["pair"] for r in measured if r["cpu_wall_ratio"] < cutoffs[r["arm"]]}
        )
        retained = [r for r in measured if r["pair"] not in excluded]
        stats = {}
        for arm in ("baseline", "candidate"):
            samples = [r for r in retained if r["arm"] == arm]
            stats[arm] = {
                metric: median(r[metric] for r in samples)
                for metric in ("wall_seconds", "cpu_seconds")
            }
            stats[arm]["cpu_wall_range"] = [
                min(r["cpu_wall_ratio"] for r in samples),
                max(r["cpu_wall_ratio"] for r in samples),
            ]
        paired = {
            index: {r["arm"]: r for r in retained if r["pair"] == index}
            for index in sorted({r["pair"] for r in retained})
        }
        deltas = {
            metric: [
                p["candidate"][metric] - p["baseline"][metric] for p in paired.values()
            ]
            for metric in ("wall_seconds", "cpu_seconds")
        }
        passed = len(paired) >= protocol["starvation"][
            "minimum_retained_pairs_per_mode"
        ] and all(
            stats["candidate"][metric] < stats["baseline"][metric]
            and median(deltas[metric]) < 0
            for metric in deltas
        )
        summary[mode] = {
            "retained_pairs": len(paired),
            "excluded_pairs": excluded,
            "starvation_cutoffs": cutoffs,
            "medians": stats,
            "wall_reduction_percent": 100
            * (
                1
                - stats["candidate"]["wall_seconds"] / stats["baseline"]["wall_seconds"]
            ),
            "cpu_reduction_percent": 100
            * (
                1 - stats["candidate"]["cpu_seconds"] / stats["baseline"]["cpu_seconds"]
            ),
            "paired_median_deltas": {
                metric: median(values) for metric, values in deltas.items()
            },
            "pairs_favoring_candidate": {
                metric: sum(value < 0 for value in values)
                for metric, values in deltas.items()
            },
            "strict_range_separation": {
                metric: max(r[metric] for r in retained if r["arm"] == "candidate")
                < min(r[metric] for r in retained if r["arm"] == "baseline")
                for metric in deltas
            },
            "output": {
                key: proof[0][key]
                for key in ("sha256", "bytes", "type_counts", "open_loop_warnings")
            },
            "gate_passed": passed,
        }
    result = {
        "recommendation": "KEEP"
        if all(s["gate_passed"] for s in summary.values())
        else "inconclusive/DROP",
        "automatic_commit": False,
        "scope": protocol["job"],
        "modes": summary,
    }
    print(json.dumps(result, indent=2))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reduce", type=Path)
    parser.add_argument("--write-summary", action="store_true")
    args = parser.parse_args()
    if args.reduce:
        reduce(args.reduce.resolve())
    result = verify()
    if args.write_summary:
        (HERE / "summary.json").write_text(
            json.dumps(result, indent=2) + "\n", encoding="utf-8"
        )
    elif (HERE / "summary.json").exists():
        require(result == read_json(HERE / "summary.json"), "saved summary drift")


if __name__ == "__main__":
    main()
