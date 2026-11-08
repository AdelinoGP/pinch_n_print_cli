"""Re-derive attribution from reduced captures; no raw tree or models needed.

--reduce writes the summary. Default verifies it. --falsify additionally checks
that changed costs, absent work counts, output drift and unclean status reject.
"""

import copy
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
MODULE = "com.core.arachne-perimeters"
COUNTERS = [
    "legacy_sign_predicates",
    "legacy_contour_vertices_presented",
    "legacy_distance_edges",
    "contexts",
    "public_queries",
    "indexed_edge_evaluations_all",
    "indexed_range_edges",
    "sign_y_records",
    "sign_unique_candidates",
    "indexed_sign_predicates",
    "indexed_contour_vertices_presented",
    "post_first_hit_predicates",
    "post_first_hit_vertices",
    "true_indexed_predicates",
]


def load(name):
    return json.loads((HERE / name).read_text(encoding="utf-8-sig"))


def capture(name):
    return [json.loads(l) for l in (HERE / name).read_text().splitlines()]


def module(events):
    summaries = [e for e in events if e["event"] == "profile_summary"]
    assert len(summaries) == 1, "exactly one profile summary required"
    modules = [
        m for m in summaries[0]["profile"]["modules"] if m["module_id"] == MODULE
    ]
    assert len(modules) == 1
    m = modules[0]
    assert m["unit"] == "fuel" and m["total_fuel"] > 0
    assert m["total_fuel"] == m["self_fuel"] + sum(
        s["self_fuel"] for s in m["scopes"]
    ), "module fuel conservation"
    return m


def output(d):
    c = d["completion"]
    assert d["exit_code"] == 0 and c["status"] == "ok" and not c["degraded"]
    assert c["fatal_error_count"] == c["non_fatal_error_count"] == 0
    assert not d["module_errors"]
    assert d["generator_markers"] == ["; wall_generator = arachne"]
    assert len(d["slice_stats"]) == 1 and d["slice_stats"][0]["layer_count"] > 0
    assert d["bytes"] > 0 and all(v > 0 for v in d["type_counts"].values())


def derive(mode, probe_events, baseline_events, disclosures):
    for d in disclosures:
        output(d)
    for es, d in [(probe_events, disclosures[1]), (baseline_events, disclosures[2])]:
        completed = [e for e in es if e["event"] == "slice_complete"]
        assert completed == [d["completion"]], "capture/disclosure completion mismatch"
    baseline_output = disclosures[0]
    for d in disclosures[1:]:
        for key in [
            "sha256",
            "bytes",
            "type_counts",
            "generator_markers",
            "open_loop_warnings",
        ]:
            assert d[key] == baseline_output[key], f"output drift: {key}"
    old = baseline_output["inventory"]
    new = disclosures[1]["inventory"]
    assert old.keys() == new.keys()
    changed = [k for k in old if old[k] != new[k]]
    assert set(changed) == {
        "pnp_cli.exe",
        "modules/arachne-perimeters/arachne-perimeters.wasm",
    }
    m, b = module(probe_events), module(baseline_events)
    scopes = {
        s["scope"].split("t49::")[1]: s
        for s in m["scopes"]
        if s["scope"].startswith("t49::")
    }

    def fuel(name, inclusive=False):
        return scopes.get(name, {}).get("total_fuel" if inclusive else "self_fuel", 0)

    def calls(name):
        return scopes.get(name, {}).get("calls", 0)

    rows = [
        json.loads(e["message"].split("T49-COUNTS ")[1])
        for e in probe_events
        if e["event"] == "module_log"
        and e.get("module_id") == MODULE
        and "T49-COUNTS " in e.get("message", "")
    ]
    assert (
        len(rows) == m["calls"] == baseline_output["slice_stats"][0]["layer_count"]
    ), "missing invocation work counts"
    assert all(
        len(row) == len(COUNTERS) and all(isinstance(n, int) and n >= 0 for n in row)
        for row in rows
    )
    counts = dict(zip(COUNTERS, map(sum, zip(*rows))))
    assert counts["public_queries"] == calls("public") > 0
    assert counts["contexts"] == calls("context") > 0
    assert (
        calls("predicate")
        == counts["legacy_sign_predicates"] + counts["indexed_sign_predicates"]
        > 0
    )
    assert sum(s["self_fuel"] for s in scopes.values()) == fuel("public", True) + fuel(
        "context", True
    ), "query/context fuel conservation"
    sign = sum(fuel(n) for n in ["sign", "predicate", "index_sign", "collect", "scan"])
    distance = sum(
        fuel(n) for n in ["full_distance", "index_distance", "seed", "range", "edge"]
    )
    wrappers = sum(fuel(n) for n in ["public", "legacy", "fallback", "linear", "other"])
    context_account = fuel("context", True) - fuel("context")
    query_account = fuel("account") - context_account
    assert sign + distance + wrappers + query_account == fuel("public", True), (
        "non-overlapping query accounting"
    )
    if mode == "ordinary":
        assert calls("legacy") == counts["public_queries"]
        assert (
            counts["legacy_distance_edges"] > 0
            and counts["legacy_contour_vertices_presented"] > 0
        )
        assert all(counts[n] == 0 for n in COUNTERS[5:])
    else:
        assert (
            calls("seed")
            == calls("index_distance")
            == calls("index_sign")
            == counts["public_queries"]
        )
        assert counts["indexed_edge_evaluations_all"] == calls("edge") > 0
        assert counts["indexed_range_edges"] > 0
        assert (
            counts["sign_y_records"]
            >= counts["sign_unique_candidates"]
            == counts["indexed_sign_predicates"]
            > 0
        )
        assert (
            0
            <= counts["post_first_hit_predicates"]
            <= counts["indexed_sign_predicates"]
        )
        assert all(counts[n] == 0 for n in COUNTERS[:3])
    # Missing fallback rows mean zero activations, not a universal guarantee.
    assert calls("fallback") == calls("linear") == calls("other") == 0
    denominator = sign + distance + wrappers
    return {
        "module_fuel": m["total_fuel"],
        "baseline_module_fuel": b["total_fuel"],
        "probe_module_fuel_delta": m["total_fuel"] - b["total_fuel"],
        "probe_module_fuel_delta_percent": 100
        * (m["total_fuel"] / b["total_fuel"] - 1),
        "public_inclusive_fuel": fuel("public", True),
        "sign_self_sum_fuel": sign,
        "distance_self_sum_fuel": distance,
        "wrapper_self_sum_fuel": wrappers,
        "query_counter_self_fuel": query_account,
        "context_inclusive_fuel": fuel("context", True),
        "sign_percent_excluding_counter_self": 100 * sign / denominator,
        "distance_percent_excluding_counter_self": 100 * distance / denominator,
        "wrapper_percent_excluding_counter_self": 100 * wrappers / denominator,
        "predicate_percent_of_sign": 100 * fuel("predicate") / sign,
        "context_percent_of_module": 100 * fuel("context", True) / m["total_fuel"],
        "indexed_seed_traversal_exact_edges": (
            counts["indexed_edge_evaluations_all"]
            - counts["indexed_range_edges"]
            - counts["public_queries"]
        )
        if mode == "accelerated"
        else 0,
        "fallback_queries": calls("fallback"),
        "counts": counts,
        "scope_rows": scopes,
        "output_sha256": baseline_output["sha256"],
        "output_bytes": baseline_output["bytes"],
        "type_counts": baseline_output["type_counts"],
        "open_loop_warnings": baseline_output["open_loop_warnings"],
        "conservation": "PASS; public leaf+self+counter = inclusive; context separate; module sum exact",
    }


def main():
    identity = load("identity.json")
    assert all(
        identity["freshness"][m]["exit_code"] == 0 for m in ["ordinary", "accelerated"]
    )
    result = {}
    for mode in ["ordinary", "accelerated"]:
        pe = capture(f"{mode}-capture.jsonl")
        be = capture(f"{mode}-baseline-profile-capture.jsonl")
        ds = [
            load(f"{mode}-{phase}-output.json")
            for phase in ["baseline", "probe", "baseline-profile"]
        ]
        assert ds[0]["inventory"] == identity["snapshots"][mode]
        result[mode] = derive(mode, pe, be, ds)
        if "--falsify" in sys.argv:
            mutants = []
            bad = copy.deepcopy(pe)
            next(
                s
                for e in bad
                if e["event"] == "profile_summary"
                for m in e["profile"]["modules"]
                if m["module_id"] == MODULE
                for s in m["scopes"]
                if s["scope"] == "t49::predicate"
            )["self_fuel"] += 1
            mutants.append((bad, ds))
            mutants.append(([e for e in pe if e["event"] != "module_log"], ds))
            bad_ds = copy.deepcopy(ds)
            bad_ds[1]["sha256"] = "tampered"
            mutants.append((pe, bad_ds))
            bad_ds = copy.deepcopy(ds)
            bad_ds[1]["completion"]["non_fatal_error_count"] = 1
            mutants.append((pe, bad_ds))
            for bad_pe, bad_ds in mutants:
                try:
                    derive(mode, bad_pe, be, bad_ds)
                except AssertionError:
                    continue
                raise AssertionError("sabotage accepted")
    assert result["ordinary"]["output_sha256"] == result["accelerated"]["output_sha256"]
    restored = load("restoration.json")
    assert (
        restored["source_identity_matches_before"]
        and restored["source_file_count_checked"] > 0
    )
    for mode in ["ordinary", "accelerated"]:
        r = restored["modes"][mode]
        assert r["freshness_exit_code"] == 0 and r["inventory_matches_before"]
        assert r["inventory"] == identity["snapshots"][mode]
    validated = load("output-validation.json")
    assert {(r["mode"], r["phase"]) for r in validated} == {
        (m, p)
        for m in ["ordinary", "accelerated"]
        for p in ["baseline", "probe", "baseline-profile"]
    }
    for row in validated:
        e = row["evidence"]
        assert e["CompletionStatus"] == "ok" and not e["Degraded"]
        assert e["FatalErrorCount"] == e["NonFatalErrorCount"] == 0
    if "--reduce" in sys.argv:
        (HERE / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    else:
        assert result == load("summary.json"), "summary does not re-derive"
    for mode, r in result.items():
        print(
            f"{mode}: sign {r['sign_percent_excluding_counter_self']:.3f}%, distance {r['distance_percent_excluding_counter_self']:.3f}%; probe module delta {r['probe_module_fuel_delta_percent']:+.3f}%; counts/output/conservation PASS"
        )
    print(
        "PASS"
        + (
            "; tampered cost, absent counts, output drift and dirty status rejected in both modes"
            if "--falsify" in sys.argv
            else ""
        )
    )


if __name__ == "__main__":
    main()
