"""Verify tracked evidence; --reduce reads existing captures, never runs slices."""

import argparse
import csv
import importlib.util
import json
import math
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
LINEAGE = HERE.parent / "t44-clip-universe-hoist"
spec = importlib.util.spec_from_file_location("t44", LINEAGE / "verify-t44.py")
lineage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lineage)
require, digest, inventory, disclosure = (
    lineage.require,
    lineage.digest,
    lineage.inventory,
    lineage.disclosure,
)


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def reduce(raw):
    arms = load(raw / "snapshots/arm-inventory.json")
    for mode, pair in arms.items():
        for arm in ("baseline", "candidate"):
            require(
                inventory(raw / f"snapshots/{mode}-{arm}") == pair[arm],
                "snapshot drift",
            )
    rows = []
    for phase in ("proof", "measure"):
        for mode in ("ordinary", "accelerated"):
            run_dir = raw / phase / mode
            with (run_dir / "rows.csv").open(
                encoding="utf-8-sig", newline=""
            ) as stream:
                for row in csv.DictReader(stream):
                    for k in (
                        "pair",
                        "warmup",
                        "threads",
                        "bytes",
                        "fatal",
                        "non_fatal",
                        "open_loop_warnings",
                        "exit_code",
                    ):
                        row[k] = int(row[k])
                    for k in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
                        row[k] = float(row[k])
                    row["degraded"] = row["degraded"].lower() == "true"
                    row["type_counts"] = json.loads(row["type_counts"])
                    stem = run_dir / row["run_id"]
                    actual = disclosure(
                        stem.with_suffix(".gcode"), stem.with_suffix(".stderr.jsonl")
                    )
                    for k in ("sha256", "bytes", "type_counts", "open_loop_warnings"):
                        require(
                            row[k] == actual[k],
                            f"CSV/output mismatch {row['run_id']}/{k}",
                        )
                    require(
                        actual["completion"]
                        == {
                            "status": row["status"],
                            "degraded": row["degraded"],
                            "fatal_error_count": row["fatal"],
                            "non_fatal_error_count": row["non_fatal"],
                        },
                        "completion mismatch",
                    )
                    row.update(
                        {
                            k: actual[k]
                            for k in ("event_counts", "shadow_warnings", "probe_lines")
                        }
                    )
                    rows.append(row)
    data = {
        "protocol": load(HERE / "protocol.json"),
        "protocol_sha256": digest(HERE / "protocol.json"),
        "config_sha256": digest(LINEAGE / "benchy-arachne.json"),
        "model_sha256": digest(HERE.parents[4] / "tmp/3dbenchy.stl"),
        "baseline_head": (raw / "snapshots/head.txt").read_text().strip(),
        "candidate_patch_sha256": digest(HERE / "candidate.patch"),
        "raw_root_optional": str(raw),
        "arms": arms,
        "rows": rows,
    }
    (HERE / "evidence.json").write_text(json.dumps(data, indent=2) + "\n")


def verify(data):
    protocol = load(HERE / "protocol.json")
    require(
        data["protocol"] == protocol
        and data["protocol_sha256"] == digest(HERE / "protocol.json"),
        "protocol identity",
    )
    require(
        data["config_sha256"] == digest(LINEAGE / "benchy-arachne.json"),
        "config identity",
    )
    require(
        data["candidate_patch_sha256"] == digest(HERE / "candidate.patch"),
        "patch identity",
    )
    require(
        data["model_sha256"]
        == "6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7",
        "model identity",
    )
    require(len(data["rows"]) == 32, "proof and measured population")
    restored = load(HERE / "restoration.json")
    for mode in protocol["modes"]:
        require(
            restored[mode]["inventory_matches_baseline"] is True
            and restored[mode]["inventory"] == data["arms"][mode]["baseline"],
            "restored distribution identity",
        )
    summary = {}
    for mode in protocol["modes"]:
        arms = data["arms"][mode]
        require(
            arms["baseline"].keys() == arms["candidate"].keys(), "snapshot file set"
        )
        changes = [
            k for k in arms["baseline"] if arms["baseline"][k] != arms["candidate"][k]
        ]
        require(
            changes == arms["changed"] == [protocol["only_arm_difference"]],
            "one-variable identity",
        )
        rows = [r for r in data["rows"] if r["mode"] == mode]
        proofs = [r for r in rows if r["phase"] == "proof"]
        measurements = [r for r in rows if r["phase"] == "measure"]
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in proofs]
            == [("baseline", 0, 1), ("candidate", 0, 1)],
            "proof order",
        )
        order = [("baseline", 0, 1), ("candidate", 0, 1)] + [
            (a, i, 0)
            for i in range(1, 7)
            for a in (("baseline", "candidate") if i % 2 else ("candidate", "baseline"))
        ]
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in measurements] == order,
            "interleaving",
        )
        output_keys = ("sha256", "bytes", "type_counts", "open_loop_warnings")
        for r in rows:
            require(
                all(r[k] == proofs[0][k] for k in output_keys), "exactness/disclosure"
            )
            require(
                r["threads"] == 12
                and r["generator"] == "arachne"
                and r["status"] == "ok"
                and not r["degraded"]
                and r["fatal"] == r["non_fatal"] == r["exit_code"] == 0,
                "completion",
            )
            require(
                r["probe_lines"] == r["shadow_warnings"] == 0
                and all(
                    r["event_counts"].get(e, 0) == 0
                    for e in ("module_complete", "stage_complete", "profile_summary")
                ),
                "uninstrumented/isolation",
            )
            require(
                all(
                    math.isfinite(r[k]) and r[k] > 0
                    for k in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio")
                ),
                "finite timings",
            )
            require(
                math.isclose(
                    r["cpu_wall_ratio"],
                    r["cpu_seconds"] / r["wall_seconds"],
                    rel_tol=1e-12,
                ),
                "ratio arithmetic",
            )
        measured = measurements[2:]
        cutoffs = {
            a: 0.75 * max(r["cpu_wall_ratio"] for r in measured if r["arm"] == a)
            for a in ("baseline", "candidate")
        }
        excluded = sorted(
            {r["pair"] for r in measured if r["cpu_wall_ratio"] < cutoffs[r["arm"]]}
        )
        retained = [r for r in measured if r["pair"] not in excluded]
        require(len(retained) >= 10, "insufficient retained pairs; no recommendation")
        stats = {
            a: {
                **{
                    k: median(r[k] for r in retained if r["arm"] == a)
                    for k in ("wall_seconds", "cpu_seconds")
                },
                "cpu_wall_range": [
                    min(r["cpu_wall_ratio"] for r in retained if r["arm"] == a),
                    max(r["cpu_wall_ratio"] for r in retained if r["arm"] == a),
                ],
            }
            for a in ("baseline", "candidate")
        }
        pairs = {
            i: {r["arm"]: r for r in retained if r["pair"] == i}
            for i in sorted({r["pair"] for r in retained})
        }
        deltas = {
            k: [p["candidate"][k] - p["baseline"][k] for p in pairs.values()]
            for k in ("wall_seconds", "cpu_seconds")
        }
        passed = all(
            stats["candidate"][k] < stats["baseline"][k] and median(deltas[k]) < 0
            for k in deltas
        )
        summary[mode] = {
            "retained_pairs": len(pairs),
            "excluded_pairs": excluded,
            "starvation_cutoffs": cutoffs,
            "medians": stats,
            "paired_median_deltas": {k: median(v) for k, v in deltas.items()},
            "reduction_percent": {
                k: 100 * (1 - stats["candidate"][k] / stats["baseline"][k])
                for k in deltas
            },
            "pairs_favoring_candidate": {
                k: sum(d < 0 for d in v) for k, v in deltas.items()
            },
            "strict_range_separation": {
                k: max(r[k] for r in retained if r["arm"] == "candidate")
                < min(r[k] for r in retained if r["arm"] == "baseline")
                for k in deltas
            },
            "output": {k: proofs[0][k] for k in output_keys},
            "gate_passed": passed,
        }
    return {
        "recommendation": "KEEP"
        if all(s["gate_passed"] for s in summary.values())
        else "DROP/inconclusive",
        "automatic_commit": False,
        "modes": summary,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reduce", type=Path)
    parser.add_argument("--write-summary", action="store_true")
    parser.add_argument("--falsify", action="store_true")
    args = parser.parse_args()
    if args.reduce:
        reduce(args.reduce.resolve())
    data = load(HERE / "evidence.json")
    result = verify(data)
    if args.falsify:
        damaged = json.loads(json.dumps(data))
        damaged["rows"][1]["sha256"] = "0" * 64
        try:
            verify(damaged)
        except RuntimeError:
            print("PASS: altered proof output rejected")
        else:
            raise RuntimeError("negative control accepted")
    if args.write_summary:
        (HERE / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    else:
        require(load(HERE / "summary.json") == result, "saved summary drift")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
