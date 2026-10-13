"""Reduce existing captures or verify the bounded experiment from tracked evidence."""

import argparse
import copy
import csv
import importlib.util
import json
import math
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "t45_reducer", HERE.parent / "t45-quartile-bbox/verify-t45.py"
)
PRECEDENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PRECEDENT)
# Reuse the unchanged raw CSV/output/inventory reducer, not its all-pairs-green verifier.
PRECEDENT.HERE = HERE
require, digest, load = PRECEDENT.require, PRECEDENT.digest, PRECEDENT.load


def artifact_name(campaign, name):
    return HERE / (name if campaign == "initial" else f"{campaign}-{name}")


def reduce_campaign(raw, campaign):
    """Read separate raw populations without replacing the initial evidence."""
    if campaign == "initial":
        PRECEDENT.reduce(raw)
        return
    arms = load(raw / "snapshots/arm-inventory.json")
    for mode, pair in arms.items():
        for arm in ("baseline", "candidate"):
            require(
                PRECEDENT.inventory(raw / f"snapshots/{mode}-{arm}") == pair[arm],
                "snapshot drift",
            )
    rows = []
    for phase in ("proof", "measure"):
        for mode in ("ordinary", "accelerated"):
            run_dir = raw / campaign / phase / mode
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
                    stem = run_dir / row["run_id"]
                    actual = PRECEDENT.disclosure(
                        stem.with_suffix(".gcode"), stem.with_suffix(".stderr.jsonl")
                    )
                    for key in ("sha256", "bytes", "type_counts", "open_loop_warnings"):
                        require(
                            row[key] == actual[key],
                            f"CSV/output mismatch: {row['run_id']}/{key}",
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
                            key: actual[key]
                            for key in (
                                "event_counts",
                                "shadow_warnings",
                                "probe_lines",
                            )
                        }
                    )
                    rows.append(row)
    data = {
        "protocol": load(HERE / "protocol.json"),
        "protocol_sha256": digest(HERE / "protocol.json"),
        "config_sha256": digest(
            HERE.parent / "t44-clip-universe-hoist/benchy-arachne.json"
        ),
        "model_sha256": digest(HERE.parents[4] / "tmp/3dbenchy.stl"),
        "baseline_head": (raw / "snapshots/head.txt")
        .read_text(encoding="utf-8")
        .strip(),
        "candidate_patch_sha256": digest(HERE / "candidate.patch"),
        "raw_root_optional": str(raw / campaign),
        "arms": arms,
        "rows": rows,
        "campaign": campaign,
        "authorization_sha256": digest(HERE / "retry-1-authorization.json"),
    }
    artifact_name(campaign, "evidence.json").write_text(
        json.dumps(data, indent=2) + "\n", encoding="utf-8"
    )


def verify(data):
    protocol = load(HERE / "protocol.json")
    template = load(HERE.parent / "t45-quartile-bbox/protocol.json")
    require(
        {k: v for k, v in protocol.items() if k != "purpose"}
        == {k: v for k, v in template.items() if k != "purpose"},
        "standing protocol changed",
    )
    require(
        data["protocol"] == protocol
        and data["protocol_sha256"] == digest(HERE / "protocol.json"),
        "protocol identity",
    )
    require(
        data["config_sha256"]
        == digest(HERE.parent / "t44-clip-universe-hoist/benchy-arachne.json"),
        "config identity",
    )
    require(
        data["candidate_patch_sha256"] == digest(HERE / "candidate.patch"),
        "candidate patch identity",
    )
    identity = load(HERE / "identity.json")
    require(
        data["model_sha256"] == identity["model_sha256"]
        and data["baseline_head"] == identity["baseline_head"],
        "capture input identity",
    )
    require(
        all(identity["freshness"][m]["exit_code"] == 0 for m in protocol["modes"]),
        "baseline freshness",
    )
    require(len(data["rows"]) == 32, "proof and measured population")
    campaign = data.get("campaign", "initial")
    require(campaign in ("initial", "retry-1"), "authorized campaign only")
    if campaign == "retry-1":
        authorization = load(HERE / "retry-1-authorization.json")
        require(
            data["authorization_sha256"] == digest(HERE / "retry-1-authorization.json"),
            "retry authorization identity",
        )
        require(
            authorization["human_request"] == "Try one more run",
            "explicit human retry request",
        )
        require(
            authorization["protocol_sha256"] == data["protocol_sha256"]
            and authorization["candidate_patch_sha256"]
            == data["candidate_patch_sha256"],
            "same candidate and protocol on retry",
        )
        require(
            authorization["preserved_initial_evidence_sha256"]
            == digest(HERE / "evidence.json")
            and authorization["preserved_initial_summary_sha256"]
            == digest(HERE / "summary.json"),
            "initial evidence preserved",
        )
        require(
            all(
                authorization["freshness"][m]["exit_code"] == 0
                for m in protocol["modes"]
            ),
            "repeat mode-specific freshness",
        )
        original = load(HERE / "evidence.json")
        require(
            data["arms"] == original["arms"], "identical arms; no substituted candidate"
        )
    restoration = load(HERE / "restoration.json")
    summary = {}
    for mode in protocol["modes"]:
        arms = data["arms"][mode]
        require(
            arms["baseline"] == identity["snapshots"][mode],
            "preserved baseline snapshot identity",
        )
        require(
            arms["baseline"].keys() == arms["candidate"].keys(), "snapshot file set"
        )
        changes = [
            k for k in arms["baseline"] if arms["baseline"][k] != arms["candidate"][k]
        ]
        require(
            changes == arms["changed"] == [protocol["only_arm_difference"]],
            "isolated guest identity",
        )
        require(
            restoration[mode]["inventory_matches_baseline"] is True
            and restoration[mode]["inventory"] == arms["baseline"],
            "active distribution restoration",
        )
        rows = [r for r in data["rows"] if r["mode"] == mode]
        proofs = [r for r in rows if r["phase"] == "proof"]
        measurements = [r for r in rows if r["phase"] == "measure"]
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in proofs]
            == [("baseline", 0, 1), ("candidate", 0, 1)],
            "proof order",
        )
        expected = [("baseline", 0, 1), ("candidate", 0, 1)] + [
            (arm, i, 0)
            for i in range(1, protocol["measured_pairs_per_mode"] + 1)
            for arm in (
                ("baseline", "candidate") if i % 2 else ("candidate", "baseline")
            )
        ]
        require(
            [(r["arm"], r["pair"], r["warmup"]) for r in measurements] == expected,
            "interleaving",
        )
        output_keys = ("sha256", "bytes", "type_counts", "open_loop_warnings")
        for row in rows:
            require(
                all(row[k] == proofs[0][k] for k in output_keys), "exactness/disclosure"
            )
            if campaign == "retry-1":
                original_proof = next(
                    r
                    for r in original["rows"]
                    if r["mode"] == mode and r["phase"] == "proof"
                )
                require(
                    all(row[k] == original_proof[k] for k in output_keys),
                    "same real-output proof across campaigns",
                )
            require(
                row["threads"] == protocol["threads"]
                and row["generator"] == "arachne"
                and row["status"] == "ok"
                and not row["degraded"]
                and row["fatal"] == row["non_fatal"] == row["exit_code"] == 0,
                "completion",
            )
            require(
                row["probe_lines"] == row["shadow_warnings"] == 0
                and all(
                    row["event_counts"].get(e, 0) == 0
                    for e in ("module_complete", "stage_complete", "profile_summary")
                ),
                "probe-free timing/isolation",
            )
            require(
                all(
                    math.isfinite(row[k]) and row[k] > 0
                    for k in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio")
                ),
                "finite timing",
            )
            require(
                math.isclose(
                    row["cpu_wall_ratio"],
                    row["cpu_seconds"] / row["wall_seconds"],
                    rel_tol=1e-12,
                ),
                "ratio arithmetic",
            )
        measured = measurements[2:]
        cutoffs = {
            arm: protocol["starvation"]["cutoff_fraction_of_best_cpu_wall_ratio"]
            * max(r["cpu_wall_ratio"] for r in measured if r["arm"] == arm)
            for arm in ("baseline", "candidate")
        }
        excluded = sorted(
            {r["pair"] for r in measured if r["cpu_wall_ratio"] < cutoffs[r["arm"]]}
        )
        retained = [r for r in measured if r["pair"] not in excluded]
        pairs = {
            i: {r["arm"]: r for r in retained if r["pair"] == i}
            for i in sorted({r["pair"] for r in retained})
        }
        enough = len(pairs) >= protocol["starvation"]["minimum_retained_pairs_per_mode"]
        stats = (
            {
                arm: {
                    key: median(r[key] for r in retained if r["arm"] == arm)
                    for key in ("wall_seconds", "cpu_seconds")
                }
                for arm in ("baseline", "candidate")
            }
            if pairs
            else None
        )
        deltas = (
            {
                key: median(
                    p["candidate"][key] - p["baseline"][key] for p in pairs.values()
                )
                for key in ("wall_seconds", "cpu_seconds")
            }
            if pairs
            else None
        )
        passed = enough and all(
            stats["candidate"][key] < stats["baseline"][key] and deltas[key] < 0
            for key in deltas
        )
        summary[mode] = {
            "retained_pairs": len(pairs),
            "excluded_pairs": excluded,
            "starvation_cutoffs": cutoffs,
            "minimum_pairs_satisfied": enough,
            "descriptive_retained_medians_not_acceptance": stats,
            "descriptive_paired_median_deltas_not_acceptance": deltas,
            "output": {k: proofs[0][k] for k in output_keys},
            "gate_passed": passed,
            "reason": "insufficient retained pairs; no automatic retry"
            if not enough
            else (
                "paired wall/CPU clauses pass"
                if passed
                else "paired wall/CPU clauses fail"
            ),
        }
    require(
        summary["ordinary"]["output"] == summary["accelerated"]["output"],
        "cross-mode current output",
    )
    return {
        "recommendation": "KEEP"
        if all(s["gate_passed"] for s in summary.values())
        else (
            "DROP"
            if all(s["minimum_pairs_satisfied"] for s in summary.values())
            else "DROP/inconclusive"
        ),
        "automatic_commit": False,
        "automatic_retry": False,
        "modes": summary,
    }


def accounting():
    result = {}
    for mode in ("ordinary", "accelerated"):
        captures = {
            phase: load(HERE / f"{mode}-{phase}-profile.json")
            for phase in ("control", "probe")
        }
        modules = {}
        for phase, capture in captures.items():
            require(capture["exit_code"] == 0, "diagnostic exit")
            completions = [
                e for e in capture["events"] if e["event"] == "slice_complete"
            ]
            require(len(completions) == 1, "diagnostic completion cardinality")
            c = completions[0]
            require(
                c["status"] == "ok"
                and not c["degraded"]
                and c["fatal_error_count"] == c["non_fatal_error_count"] == 0,
                "diagnostic clean completion",
            )
            profiles = [e for e in capture["events"] if e["event"] == "profile_summary"]
            require(len(profiles) == 1, "profile population")
            modules[phase] = next(
                m
                for m in profiles[0]["profile"]["modules"]
                if m["module_id"] == "com.core.arachne-perimeters"
            )
            m = modules[phase]
            require(
                m["self_fuel"] + sum(s["self_fuel"] for s in m["scopes"])
                == m["total_fuel"],
                "non-overlapping module fuel conservation",
            )
        require(
            captures["control"]["sha256"] == captures["probe"]["sha256"],
            "probe output identity",
        )
        before, after = captures["control"]["snapshot"], captures["probe"]["snapshot"]
        require(
            before.keys() == after.keys()
            and [k for k in before if before[k] != after[k]]
            == ["modules/arachne-perimeters/arachne-perimeters.wasm"],
            "probe isolated guest",
        )
        logs = [e for e in captures["probe"]["events"] if e["event"] == "module_log"]
        require(bool(logs), "cache accounting population")
        counts = [
            json.loads(e["message"].removeprefix("PERF-T51-COUNTS ")) for e in logs
        ]
        require(
            all(
                len(row) == 6 and all(isinstance(n, int) and n >= 0 for n in row)
                for row in counts
            ),
            "cache count schema",
        )
        totals = [sum(row[i] for row in counts) for i in range(6)]
        scopes = {s["scope"]: s for s in modules["probe"]["scopes"]}
        cache, bookkeeping = scopes["t51::cache_construction"], scopes["t51::account"]
        require(
            totals[0] > 0 and totals[2] > 0 and totals[5] > 0,
            "construction/query work exercised",
        )
        require(
            cache["calls"] == totals[0]
            and cache["self_fuel"] == cache["total_fuel"] > 0,
            "isolated cache construction fuel",
        )
        require(
            bookkeeping["calls"] == totals[0] * 5 + totals[5],
            "separate bookkeeping call conservation",
        )
        require(
            all(row[0] == 1 for row in counts),
            "one context per observed invocation for max-context storage",
        )
        result[mode] = {
            "contexts": totals[0],
            "contours": totals[1],
            "prepared_vertices": totals[2],
            "retained_capacity_bytes_summed_over_contexts_not_peak_rss": totals[3],
            "backing_buffers_summed_over_contexts_not_allocator_calls": totals[4],
            "public_queries": totals[5],
            "public_queries_per_context_mean": totals[5] / totals[0],
            "maximum_single_context_retained_capacity_bytes": max(
                row[3] for row in counts
            ),
            "cache_construction_self_fuel": cache["self_fuel"],
            "cache_construction_share_of_probed_module_percent": 100
            * cache["self_fuel"]
            / modules["probe"]["total_fuel"],
            "bookkeeping_self_fuel": bookkeeping["self_fuel"],
            "control_module_fuel": modules["control"]["total_fuel"],
            "probe_module_fuel": modules["probe"]["total_fuel"],
            "probe_observer_delta_percent": 100
            * (modules["probe"]["total_fuel"] / modules["control"]["total_fuel"] - 1),
        }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reduce", type=Path)
    parser.add_argument("--write-summary", action="store_true")
    parser.add_argument("--falsify", action="store_true")
    parser.add_argument("--campaign", choices=("initial", "retry-1"), default="initial")
    args = parser.parse_args()
    if args.reduce:
        reduce_campaign(args.reduce.resolve(), args.campaign)
    data = load(artifact_name(args.campaign, "evidence.json"))
    require(data.get("campaign", "initial") == args.campaign, "campaign selection")
    validation = load(HERE / "validation.json")
    require(
        validation["candidate_patch_sha256"] == digest(HERE / "candidate.patch"),
        "gate candidate identity",
    )
    require(
        set(validation["gates"])
        == {"workspace-build", "clippy", "literals", "test-quality"},
        "gate population",
    )
    for gate in validation["gates"].values():
        require(
            gate["observed_checked_exit_code"] == 0
            and gate["log_sha256"] == digest(HERE / gate["log"]),
            "archived gate status/log identity",
        )
    require(
        "--all-targets" in validation["gates"]["clippy"]["command"],
        "all-targets Clippy witness",
    )
    cleanup = load(HERE / "cleanup.json")
    require(
        cleanup["human_decision"] == "drop"
        and cleanup["candidate_patch_sha256"] == digest(HERE / "candidate.patch")
        and cleanup["rebuild_matches_archived_baseline"] is True
        and cleanup["patch_reapplicable_to_restored_tree"] is True
        and cleanup["automatic_commit"] is False,
        "cleanup record",
    )
    require(
        all(
            cleanup["freshness"][m]["exit_code"] == 0
            for m in ("ordinary", "accelerated")
        )
        and all(
            record["matches_head_and_pre_experiment"] is True
            for record in cleanup["source"].values()
        ),
        "cleanup freshness/source record",
    )
    suite_log = (HERE / "cleanup-restored-suite.log").read_text(encoding="utf-8")
    require(
        "3 passed" in suite_log and "0 failed" in suite_log,
        "restored original suite log",
    )
    result = verify(data)
    # Bind diagnostic output to the fresh probe-free real-output proofs.
    for mode in ("ordinary", "accelerated"):
        for phase in ("control", "probe"):
            capture = load(HERE / f"{mode}-{phase}-profile.json")
            require(
                capture["sha256"] == result["modes"][mode]["output"]["sha256"],
                "diagnostic/current proof identity",
            )
    result["construction_accounting"] = accounting()
    if args.falsify:
        for name, mutate in [
            ("proof drift", lambda d: d["rows"][1].update(sha256="0" * 64)),
            ("dirty status", lambda d: d["rows"][2].update(non_fatal=1)),
            ("ratio tamper", lambda d: d["rows"][4].update(cpu_wall_ratio=999.0)),
            (
                "host substitution",
                lambda d: d["arms"]["ordinary"]["candidate"].update(
                    {"pnp_cli.exe": "0" * 64}
                ),
            ),
        ]:
            damaged = copy.deepcopy(data)
            mutate(damaged)
            try:
                verify(damaged)
            except (RuntimeError, ValueError):
                print(f"PASS: rejects {name}")
            else:
                raise RuntimeError(f"negative control accepted: {name}")
        # Synthetic timing mutations must falsify the gate, regardless of whether
        # the selected real campaign itself has enough retained pairs.
        starved = copy.deepcopy(data)
        for row in starved["rows"]:
            if (
                row["phase"] == "measure"
                and not row["warmup"]
                and row["arm"] == "candidate"
                and row["pair"] <= 5
            ):
                row["cpu_seconds"] *= 0.001
                row["cpu_wall_ratio"] = row["cpu_seconds"] / row["wall_seconds"]
        rejected = verify(starved)
        require(
            rejected["recommendation"] != "KEEP"
            and all(
                not m["minimum_pairs_satisfied"] and not m["gate_passed"]
                for m in rejected["modes"].values()
            ),
            "insufficient pairs must never become KEEP",
        )
        print("PASS: insufficient-pair negative control cannot pass")
        cpu_only = copy.deepcopy(data)
        for row in cpu_only["rows"]:
            if row["phase"] == "measure" and not row["warmup"]:
                row["wall_seconds"] = 10.0 if row["arm"] == "baseline" else 11.0
                row["cpu_seconds"] = 50.0 if row["arm"] == "baseline" else 45.0
                row["cpu_wall_ratio"] = row["cpu_seconds"] / row["wall_seconds"]
        rejected = verify(cpu_only)
        require(
            rejected["recommendation"] == "DROP"
            and all(
                m["minimum_pairs_satisfied"] and not m["gate_passed"]
                for m in rejected["modes"].values()
            ),
            "CPU-only win must not override wall",
        )
        print("PASS: CPU-only negative control cannot pass")
    if args.write_summary:
        artifact_name(args.campaign, "summary.json").write_text(
            json.dumps(result, indent=2) + "\n", encoding="utf-8"
        )
    else:
        require(
            load(artifact_name(args.campaign, "summary.json")) == result,
            "saved summary drift",
        )
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
