"""Reduce campaign raw data; all report numbers derive from portable results."""

import json
import hashlib
import statistics as st
from collections import defaultdict
from pathlib import Path

from campaign import HERE, REPO, RAW, cases, sha

RESULTS = HERE / "results"
OPS = {0: "offset", 1: "difference", 2: "union", 3: "intersection", 4: "xor"}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def save(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def geometry(block):
    tokens = iter(
        " ".join(line.split("#")[0] for line in block.splitlines()[1:]).split()
    )
    verts = paths = rings = 0
    for token in tokens:
        if token == "arg":
            next(tokens)
            next(tokens)
        elif token in ("subj", "clip"):
            n = int(next(tokens))
            paths += n
            for _ in range(n):
                nr = int(next(tokens))
                rings += nr
                for _ in range(nr):
                    nv = int(next(tokens))
                    verts += nv
                    for _ in range(nv * 2):
                        next(tokens)
        else:
            assert token == "end", token
    return {"vertices": verts, "paths": paths, "rings": rings}


def reduce_raw():
    manifest = load(RAW / "manifest.json")
    assert manifest["corpus"] == {
        p.relative_to(HERE).as_posix(): sha(p) for p, _, _ in cases()
    }
    for order in ("forward", "reverse"):
        rows = []
        for path, index, block in cases():
            engines = {}
            for engine in ("rust", "cpp"):
                raw = RAW / order / path.stem / str(index) / (engine + ".jsonl")
                row = load(raw)
                assert (
                    len(row["repeat_times_ns"]) == 5
                    and min(row["repeat_times_ns"]) >= 0
                )
                engines[engine] = {
                    "median_ns": st.median(row["repeat_times_ns"]),
                    "zero_repeats": row["repeat_times_ns"].count(0),
                    **{
                        k: row[k]
                        for k in ("out_paths", "out_rings", "out_verts", "out_hash")
                    },
                    "raw_sha256": sha(raw),
                }
                header = block.splitlines()[0].split()
                assert [row["site"], str(row["layer"]), str(row["opcode"])] == header[
                    1:4
                ]
            rows.append(
                {
                    "file": path.relative_to(HERE).as_posix(),
                    "case_index": index,
                    "site": header[1],
                    "layer": int(header[2]),
                    "opcode": int(header[3]),
                    "input": geometry(block),
                    **engines,
                }
            )
        save(RESULTS / (order + ".json"), rows)
    seal = manifest["seal"]
    save(
        RESULTS / "provenance.json",
        {
            "seal_sha256": manifest["seal_sha256"],
            "corpus": manifest["corpus"],
            "reduced_results_sha256": {
                order: hashlib.sha256(
                    json.dumps(
                        load(RESULTS / (order + ".json")), sort_keys=True
                    ).encode()
                ).hexdigest()
                for order in ("forward", "reverse")
            },
            "case_count": manifest["case_count"],
            "protocol": manifest["protocol"],
            **{
                k: seal[k]
                for k in (
                    "rustc",
                    "g++",
                    "commands",
                    "binaries",
                    "driver_sources",
                    "crate_package_checksum",
                    "cpp_version",
                    "lock_sha256",
                    "resolved_clipper",
                    "crate_inventory_sha256",
                )
            },
        },
    )


def metrics(rows):
    med = {e: st.median(r[e]["median_ns"] for r in rows) for e in ("rust", "cpp")}
    ratios = [
        r["rust"]["median_ns"] / r["cpp"]["median_ns"]
        for r in rows
        if r["cpp"]["median_ns"]
    ]
    sizes = sorted(rows, key=lambda r: r["input"]["vertices"])
    strata = []
    for part in range(3):
        subset = sizes[len(sizes) * part // 3 : len(sizes) * (part + 1) // 3]
        if subset:
            vals = [
                r["rust"]["median_ns"] / r["cpp"]["median_ns"]
                for r in subset
                if r["cpp"]["median_ns"]
            ]
            strata.append(
                {
                    "cases": len(subset),
                    "input_vertices_min": subset[0]["input"]["vertices"],
                    "input_vertices_max": subset[-1]["input"]["vertices"],
                    "median_case_ratio": st.median(vals) if vals else None,
                    "rust_median_ns": st.median(r["rust"]["median_ns"] for r in subset),
                    "cpp_median_ns": st.median(r["cpp"]["median_ns"] for r in subset),
                }
            )
    return {
        "cases": len(rows),
        "medians_ns": med,
        "ratio": med["rust"] / med["cpp"] if med["cpp"] else None,
        "case_ratio_min_median_max": [min(ratios), st.median(ratios), max(ratios)]
        if ratios
        else None,
        "undefined_case_ratios": len(rows) - len(ratios),
        "zero_repeats": {
            e: sum(r[e]["zero_repeats"] for r in rows) for e in ("rust", "cpp")
        },
        "hash_matches": sum(
            r["rust"]["out_hash"] == r["cpp"]["out_hash"] for r in rows
        ),
        "shape_matches": sum(
            all(
                r["rust"][k] == r["cpp"][k]
                for k in ("out_paths", "out_rings", "out_verts")
            )
            for r in rows
        ),
        "input_size_thirds": strata,
    }


def summarize(rows):
    groups = defaultdict(list)
    for r in rows:
        for key in (
            "all",
            "opcode/" + OPS[r["opcode"]],
            "site/" + r["site"],
            "file/" + r["file"],
            "file_opcode/" + r["file"] + "/" + OPS[r["opcode"]],
            "file_site/" + r["file"] + "/" + r["site"],
        ):
            groups[key].append(r)
    result = {k: metrics(v) for k, v in sorted(groups.items())}
    for key, group in groups.items():
        strata = defaultdict(list)
        for row in group:
            strata[(row["file"], row["opcode"])].append(row)
        weighted = [
            (len(subset), metrics(subset)["ratio"]) for subset in strata.values()
        ]
        assert all(ratio is not None for _, ratio in weighted)
        result[key]["case_count_weighted_stratum_ratio"] = sum(
            n * ratio for n, ratio in weighted
        ) / len(group)
    # Explicit case-count weighting across file/opcode strata, not production frequency.
    strata = [v for k, v in result.items() if k.startswith("file_opcode/")]
    result["all"]["case_count_weighted_file_opcode_ratio"] = sum(
        v["ratio"] * v["cases"] for v in strata
    ) / len(rows)
    return result


def scope_mapping(summaries):
    result = {}
    for model in ("benchy", "base"):
        p60 = (
            HERE.parent
            / "t60-host-native/captures"
            / (model + "-classic-off-profile.json")
        )
        p68 = (
            HERE.parent
            / "t68-host-roundtrips/captures"
            / (model + "-classic-off-profile.json")
        )
        t60, t68 = load(p60), load(p68)
        totals = defaultdict(int)
        owners = []
        # Booleans: sampled per-opcode ratios weighted by sampled case counts.
        # Historical sink does not preserve the boolean opcode mix.
        ratios = {}
        for order, summary in summaries.items():
            prefix = "file_opcode/corpus/t68sites/" + model
            bools = [
                v
                for k, v in summary.items()
                if k.startswith(prefix) and not k.endswith("/offset")
            ]
            ratios[order] = {
                "offset": summary[prefix + "-t68sites.corpus/offset"]["ratio"],
                "clip_polygons": sum(v["cases"] * v["ratio"] for v in bools)
                / sum(v["cases"] for v in bools),
            }
            ratios[order]["closing_ex"] = ratios[order]["offset"]
        for owner in t60["owners"]:
            scopes = []
            for scope in owner["scopes"]:
                name = scope["scope"].split("::")[-1]
                ns = scope["total_wall_ns"]
                totals[name] += ns
                selected = {}
                for order, ratios_for_order in ratios.items():
                    ratio = ratios_for_order[name]
                    prefix = "file_opcode/corpus/t68sites/" + model
                    if (
                        name == "clip_polygons"
                        and owner["owner"] == "probe:d:classic-perimeters"
                    ):
                        ratio = summaries[order][
                            prefix + "-t68sites.corpus/difference"
                        ]["ratio"]
                    elif name == "clip_polygons" and owner["owner"] in (
                        "probe:derive_ordinary",
                        "probe:derive_perimeter_source",
                    ):
                        ratio = summaries[order][
                            prefix + "-region-prep.corpus/intersection"
                        ]["ratio"]
                    selected[order] = ratio
                scopes.append(
                    {
                        "scope": name,
                        "calls": scope["calls"],
                        "accumulated_seconds": ns / 1e9,
                        "ratios_used": selected,
                        "ceilings_seconds": {
                            order: ns / 1e9 * (1 - 1 / ratio)
                            for order, ratio in selected.items()
                        },
                    }
                )
            owners.append(
                {
                    "owner": owner["owner"],
                    "owner_accumulated_seconds": owner["total_wall_ns"] / 1e9,
                    "scopes": scopes,
                }
            )
        total_rows = {
            name: {
                "accumulated_seconds": ns / 1e9,
                "ceilings_seconds": {
                    order: sum(
                        s["ceilings_seconds"][order]
                        for o in owners
                        for s in o["scopes"]
                        if s["scope"] == name
                    )
                    for order in summaries
                },
            }
            for name, ns in sorted(totals.items())
        }
        cp = next(o for o in owners if o["owner"] == "probe:d:classic-perimeters")
        rp = [
            o
            for o in owners
            if o["owner"] in ("probe:derive_ordinary", "probe:derive_perimeter_source")
        ]
        coverage = {}
        for name, ns in totals.items():
            cpns = sum(
                s["accumulated_seconds"] for s in cp["scopes"] if s["scope"] == name
            )
            rpns = sum(
                s["accumulated_seconds"]
                for o in rp
                for s in o["scopes"]
                if s["scope"] == name
            )
            coverage[name] = {
                "cp_owner_fraction": cpns / (ns / 1e9),
                "rp_owner_fraction": rpns / (ns / 1e9),
            }
        result[model] = {
            "sources": {
                str(p.relative_to(REPO)).replace("\\", "/"): sha(p) for p in (p60, p68)
            },
            "ratios_used": ratios,
            "totals": total_rows,
            "owners": owners,
            "scope_owner_coverage_proxies": coverage,
            "t60_instrumented_wall_seconds": t60["metadata"]["wall_seconds"],
            "t60_phases_ms": t60.get("phases_ms", t60.get("phases")),
            "t68_instrumented_wall_seconds": t68["metadata"]["wall_seconds"],
            "t68_phases_ms": t68["phases_ms"],
            "uninstrumented_wall_seconds": None,
            "wall_gap": "Both cited captures are instrumented/profiled; no uninstrumented wall is derivable from them.",
            "t68_ops_seconds": sum(s["ops_ns"] for s in t68["host_sites"]) / 1e9,
            "t68_host_sites": [
                {
                    "site": s["site"],
                    "service": s["service"],
                    "calls": s["calls"],
                    "ops_seconds": s["ops_ns"] / 1e9,
                }
                for s in t68["host_sites"]
            ],
        }
        for order in summaries:
            ceiling = sum(row["ceilings_seconds"][order] for row in total_rows.values())
            result[model].setdefault("all_scope_ceiling_seconds", {})[order] = ceiling
            # Context only, against explicitly instrumented walls; not wall predictions.
            result[model].setdefault(
                "saturated_context_percent_of_t68_instrumented_wall", {}
            )[order] = ceiling / (12 * t68["metadata"]["wall_seconds"]) * 100
            cpceil = sum(s["ceilings_seconds"][order] for s in cp["scopes"])
            result[model].setdefault("cp_ceiling_seconds", {})[order] = cpceil
            result[model].setdefault(
                "cp_saturated_context_percent_of_t68_instrumented_wall", {}
            )[order] = cpceil / (12 * t68["metadata"]["wall_seconds"]) * 100
    return result


def build_summary():
    passes = {
        order: summarize(load(RESULTS / (order + ".json")))
        for order in ("forward", "reverse")
    }
    return {
        "passes": passes,
        "spread_absolute_ratio_points": {
            k: abs(passes["forward"][k]["ratio"] - passes["reverse"][k]["ratio"])
            for k in passes["forward"]
        },
        "scope_mapping": scope_mapping(passes),
    }


def table(summary, prefix):
    lines = [
        "| Group | Cases | Forward Rust/C++ | Reverse Rust/C++ | Absolute spread | Forward Rust / C++ median ns | Reverse Rust / C++ median ns |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for key, f in summary["passes"]["forward"].items():
        if key.startswith(prefix):
            r = summary["passes"]["reverse"][key]
            lines.append(
                f"| {key[len(prefix) :]} | {f['cases']} | {f['ratio']:.4f} | {r['ratio']:.4f} | {abs(f['ratio'] - r['ratio']):.4f} | {f['medians_ns']['rust']:.1f} / {f['medians_ns']['cpp']:.1f} | {r['medians_ns']['rust']:.1f} / {r['medians_ns']['cpp']:.1f} |"
            )
    return lines


def render(summary):
    summary = json.loads(json.dumps(summary, sort_keys=True))
    f, r = summary["passes"]["forward"], summary["passes"]["reverse"]
    prov = load(RESULTS / "provenance.json")
    lines = [
        "# t69: Clipper engine headroom campaign",
        "",
        "**Verdict: measured headroom; a value-exact Rust optimization candidate is justified, not adopted.**",
        "The measured slow opclasses are the candidate direction. No engine-internal profiling was done; no allocation, sweep, layout, or other internal cost center is inferred.",
        "",
        "## Method and reproduction",
        "",
        "`campaign.py` verifies binary/driver hashes, complete pristine Rust/C++ source inventories and the isolated lock before/after execution; no rebuild occurred. Seal/toolchain/build commands and corpus hashes are preserved in `results/provenance.json`.",
        f"All {prov['case_count']} cases from all four t68-site files and all four t55 files ran twice: complete file/case order then globally reversed file/case order. A fresh driver process per case excludes inter-case engine state; forward runs Rust then C++, reverse C++ then Rust. Each driver performs one warmup and five timed repeats. No CPU pinning or process-level replicated A/B; order and engine precedence change together, so spread is descriptive, not a confidence interval.",
        "Case outputs are resumable under `.local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t69-campaign/{forward,reverse}/<file-stem>/<original-index>/{rust,cpp}.jsonl`; per-case timeout is 180 seconds. Parsing/flattening, output traversal/hash and output-tree destruction are outside the engine timer; engine creation/add/execute/destruction are inside (HARNESS.md).",
        "The first attempt stopped on a zero-duration repeat in an empty boolean case; the runner was corrected to retain clock-resolution zeros, resumed without discarding completed cases, and completed. The rejected initial response was not saved; that one engine/case was re-invoked. No timing-driven exclusions.",
        "Commands (repository root; long runner detached, tee logs):",
        "",
        "```bash",
        "set -o pipefail",
        "python -u docs/specs/perf-vs-orca/evidence/t69-clipper-engine/campaign.py 2>&1 | tee .local-artifacts/perf-vs-orca/t69-clipper-engine/raw/t69-campaign.log",
        "python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/analyze.py",
        "python docs/specs/perf-vs-orca/evidence/t69-clipper-engine/verify.py",
        "```",
        "",
        "All numbers below derive from `results/{forward,reverse,summary,provenance}.json`. Case time = median of the five timed repeats. Each table engine time = median across those case medians; ratio = Rust median / C++ median. Spread = absolute difference of the two ratios, in ratio points. These are shape-sample statistics, not production frequency weights. The t55 ordinary/accelerated duplicates remain; they are not independent workloads.",
        "",
        "## Headline opclasses",
        "",
    ]
    lines += table(summary, "opcode/")
    lines += ["", "## Sites (pooled across corpus files)", ""] + table(summary, "site/")
    lines += ["", "## Corpus files", ""] + table(summary, "file/")
    lines += ["", "## Corpus × opcode", ""] + table(summary, "file_opcode/")
    lines += ["", "## Corpus × site", ""] + table(summary, "file_site/")
    lines += [
        "",
        "## Case-count-weighted aggregates",
        "",
        "Within each group below, partition by corpus file/opcode and take the arithmetic mean of stratum ratios weighted by stratum case counts. This is deliberately distinct from the pooled median ratio above; neither is a production-time weight.",
        "",
        "| Group | Forward weighted ratio | Reverse weighted ratio | Absolute spread |",
        "|---|---:|---:|---:|",
    ]
    for key in f:
        if key == "all" or key.startswith(("opcode/", "site/", "file/")):
            a = f[key]["case_count_weighted_stratum_ratio"]
            b = r[key]["case_count_weighted_stratum_ratio"]
            lines.append(f"| {key} | {a:.4f} | {b:.4f} | {abs(a - b):.4f} |")
    lines += [
        "",
        f"Case-count-weighted mean of file/opcode ratios: forward **{f['all']['case_count_weighted_file_opcode_ratio']:.4f}**, reverse **{r['all']['case_count_weighted_file_opcode_ratio']:.4f}**. Weights are sampled case counts, not invocation frequencies or time. Pooled median ratios are separately {f['all']['ratio']:.4f} / {r['all']['ratio']:.4f}.",
        "",
        "## Engine times and scaling",
        "",
        "| Opclass | Forward Rust / C++ ns | Reverse Rust / C++ ns | Forward case ratio min / median / max | Reverse case ratio min / median / max |",
        "|---|---:|---:|---|---|",
    ]
    for key in f:
        if key.startswith("opcode/"):
            a, b = f[key], r[key]
            dist = lambda m: " / ".join(
                f"{x:.4f}" for x in m["case_ratio_min_median_max"]
            )
            lines.append(
                f"| {key[7:]} | {a['medians_ns']['rust']:.1f} / {a['medians_ns']['cpp']:.1f} | {b['medians_ns']['rust']:.1f} / {b['medians_ns']['cpp']:.1f} | {dist(a)} | {dist(b)} |"
            )
    lines += [
        "",
        "| Pass / opclass | Size third | Cases | Input vertex range | Median case ratio | Rust / C++ median ns |",
        "|---|---|---:|---|---:|---:|",
    ]
    for order, groups in summary["passes"].items():
        for key, m in groups.items():
            if key.startswith("opcode/"):
                for i, s in enumerate(m["input_size_thirds"]):
                    ratio = (
                        "undefined"
                        if s["median_case_ratio"] is None
                        else f"{s['median_case_ratio']:.4f}"
                    )
                    lines.append(
                        f"| {order} / {key[7:]} | {('small', 'middle', 'large')[i]} | {s['cases']} | {s['input_vertices_min']}–{s['input_vertices_max']} | {ratio} | {s['rust_median_ns']:.1f} / {s['cpp_median_ns']:.1f} |"
                    )
    lines += [
        "",
        "Median statistics are resistant to a few very slow cases; the per-case distributions and input-size thirds disclose shape dependence and timer-floor extremes rather than asserting a fitted complexity law. A tiny-case extreme is not a production speedup. Per-case medians and input path/ring/vertex counts remain in the results files.",
        "",
    ]
    for order, groups in summary["passes"].items():
        m = groups["all"]
        lines.append(
            f"- {order}: zero repeats Rust {m['zero_repeats']['rust']}, C++ {m['zero_repeats']['cpp']}; undefined case ratios (zero C++ median) {m['undefined_case_ratios']}. They remain in group medians and agreement denominators; undefined individual ratios alone are omitted from the ratio distribution."
        )
    lines += [
        "",
        "## Cross-engine value agreement",
        "",
        "Ordered FNV coordinate hash is not geometric equivalence: rotation/orientation/component order differences can disagree; collisions remain possible. Shape counts are checked separately. Known Rust 1.1.0 versus C++ 1.5.2 differences (ticket 31) are disclosed, not root-caused. This is NOT production byte-identity qualification.",
        "",
        "| Corpus / opcode | Cases | Forward hash matches | Reverse hash matches | Forward shape matches | Reverse shape matches |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for key in f:
        if key.startswith("file_opcode/"):
            a, b = f[key], r[key]
            lines.append(
                f"| {key[12:]} | {a['cases']} | {a['hash_matches']} | {b['hash_matches']} | {a['shape_matches']} | {b['shape_matches']} |"
            )
    lines.append(
        f"\nTotal per pass: {f['all']['hash_matches']}/{f['all']['cases']} forward and {r['all']['hash_matches']}/{r['all']['cases']} reverse ordered-hash matches; divergences {f['all']['cases'] - f['all']['hash_matches']} / {r['all']['cases'] - r['all']['hash_matches']}."
    )
    lines += [
        "",
        "| Divergent case (original index) | Site / layer / opcode | Rust paths/rings/verts | C++ paths/rings/verts | Pass |",
        "|---|---|---|---|---|",
    ]
    for order in ("forward", "reverse"):
        for row in load(RESULTS / (order + ".json")):
            if row["rust"]["out_hash"] != row["cpp"]["out_hash"]:
                counts = lambda e: "/".join(
                    str(row[e][k]) for k in ("out_paths", "out_rings", "out_verts")
                )
                lines.append(
                    f"| {row['file']} #{row['case_index']} | {row['site']} / {row['layer']} / {OPS[row['opcode']]} | {counts('rust')} | {counts('cpp')} | {order} |"
                )
    lines += [
        "",
        "## Historical scope mapping: idealized accumulated-time ceilings",
        "",
        "Re-derived now by `scope_mapping` in `analyze.py` from committed `../t60-host-native/captures/{benchy,base}-classic-off-profile.json`, summing each owner’s `scopes[].total_wall_ns` (not owner live spans). Both source hashes and full owner/scope/call totals are in `results/summary.json` → `scope_mapping`. These are accumulated engine-scope thread-time, NOT run wall.",
        "For each owner/scope: idealized removable seconds = accumulated seconds × (1 − 1 / measured ratio); scope totals sum those owner ceilings. Offset uses the same model’s classic-site offset ratio; closing_ex assumes that ratio transfers to its composed offsets. The CP clip scope uses that model’s difference ratio; the two RP derivation clip scopes use its intersection ratio. Other clip owners use a case-count-weighted mean of that model’s sampled boolean-opcode ratios (CP difference + RP intersection). Their historical opcode/time mix is unavailable, so this mapping is conditional, not an identified production gain. Every owner/scope ratio used is retained in `results/summary.json`. Union-only t55 is NOT assigned to supports-off scope totals.",
        "Only the classic-perimeters per-layer host-service family is directly shape-represented by the CP corpus. Historical owner-share fractions below are coverage proxies, not captured-time fractions: sparse sampled layers do not measure the fraction of owner time captured. RP inputs cover only intersection; the historical closing/offset region-preparation workload was NOT captured. Any offset/closing ceiling for such owners assumes per-opclass ratio transfer within that opclass. In the actual t60 owner table the two `probe:derive_*` owners contain clip_polygons only; the large historical closing scope is `host:slice`. Do not relabel it as a directly captured RP workload. All other owners likewise require transfer assumptions. Ticket 67 changed preparation sharing after t60; historical totals are not a current census.",
        "",
        "| Model / scope | Accumulated s | Forward idealized removable s | Reverse idealized removable s | CP owner fraction | RP owner fraction (intersection shapes only) |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for model, mapping in summary["scope_mapping"].items():
        for name, total in mapping["totals"].items():
            c = mapping["scope_owner_coverage_proxies"][name]
            lines.append(
                f"| {model} / {name} | {total['accumulated_seconds']:.6f} | {total['ceilings_seconds']['forward']:.6f} | {total['ceilings_seconds']['reverse']:.6f} | {100 * c['cp_owner_fraction']:.4f}% | {100 * c['rp_owner_fraction']:.4f}% |"
            )
    lines += [
        "",
        "### Per-owner/scope re-derivation",
        "",
        "| Model / owner | Scope | Accumulated s | Forward ceiling s | Reverse ceiling s |",
        "|---|---|---:|---:|---:|",
    ]
    for model, mapping in summary["scope_mapping"].items():
        for owner in mapping["owners"]:
            for s in owner["scopes"]:
                lines.append(
                    f"| {model} / {owner['owner']} | {s['scope']} | {s['accumulated_seconds']:.6f} | {s['ceilings_seconds']['forward']:.6f} | {s['ceilings_seconds']['reverse']:.6f} |"
                )
    lines += [
        "",
        "### Relevant owner totals and wall relevance",
        "",
        "| Model | CP owner accumulated s | Ordinary derivation s | Perimeter-source derivation s | All-scope ceiling forward / reverse s | CP-only ceiling forward / reverse s |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for model, m in summary["scope_mapping"].items():
        owners = {o["owner"]: o["owner_accumulated_seconds"] for o in m["owners"]}
        a, b = m["all_scope_ceiling_seconds"], m["cp_ceiling_seconds"]
        lines.append(
            f"| {model} | {owners['probe:d:classic-perimeters']:.6f} | {owners['probe:derive_ordinary']:.6f} | {owners['probe:derive_perimeter_source']:.6f} | {a['forward']:.6f} / {a['reverse']:.6f} | {b['forward']:.6f} / {b['reverse']:.6f} |"
        )
    lines += [
        "",
        "**Uninstrumented-wall gap:** the requested t60/t68 source files do not contain uninstrumented runs. t68 metadata’s command explicitly has `--instrument-stderr --profile --profile-verbose`; both FINDINGS files say no uninstrumented timing ran. It would be false to present their walls/phases as uninstrumented. No slicer was run in this campaign.",
        "",
        "| Model | t60 instrumented process wall s | t68 instrumented process wall s | t68 per-layer phase s | CP-only saturated context % forward / reverse | All-scope saturated context % forward / reverse |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for model, m in summary["scope_mapping"].items():
        a, b = (
            m["cp_saturated_context_percent_of_t68_instrumented_wall"],
            m["saturated_context_percent_of_t68_instrumented_wall"],
        )
        lines.append(
            f"| {model} | {m['t60_instrumented_wall_seconds']:.7f} | {m['t68_instrumented_wall_seconds']:.7f} | {m['t68_phases_ms']['per_layer'] / 1000:.3f} | {a['forward']:.4f} / {a['reverse']:.4f} | {b['forward']:.4f} / {b['reverse']:.4f} |"
        )
    lines += [
        "",
        "Saturated context = idealized accumulated ceiling ÷ 12 ÷ the explicitly instrumented t68 wall. It is an occupancy-style scale comparison, NOT a wall saving, lower bound, forecast, or gate pass. Perfectly serialized transfer would instead divide accumulated ceiling by wall without 12; neither model establishes scheduling/critical-path transfer. Different compiler backends, runtime allocators, engine revisions, temporal machine load and historical instrument overhead confound transfer. No ordinary/accelerated or wasm speedup was measured.",
        "",
        "## Where the headroom lives and verdict",
        "",
    ]
    for prefix, title in [("opcode/", "Opclass"), ("site/", "Site")]:
        for order, groups in summary["passes"].items():
            ranked = sorted(
                (
                    (k[len(prefix) :], v["ratio"])
                    for k, v in groups.items()
                    if k.startswith(prefix)
                ),
                key=lambda p: p[1],
                reverse=True,
            )
            lines.append(
                f"- {title} ranking ({order}): "
                + " > ".join(f"{k} {v:.4f}" for k, v in ranked)
                + "."
            )
    for model, m in summary["scope_mapping"].items():
        for order in ("forward", "reverse"):
            ranked = sorted(
                m["totals"].items(),
                key=lambda p: p[1]["ceilings_seconds"][order],
                reverse=True,
            )
            lines.append(
                f"- {model} {order} mapped ceiling ranking: "
                + " > ".join(
                    f"{name} {v['ceilings_seconds'][order]:.6f} s" for name, v in ranked
                )
                + "; accumulated, conditional."
            )
    lines += [
        "",
        "The dominant measured engine opclasses are materially slower than C++ in both orders; the mapped accumulated ceiling is not a below-floor round-trip-overhead lead. The [map’s t68 conclusion and Decisions](../../map.md#decisions-so-far) treat ≤0.80% serial wall relevance as below-gate; its gap-budget Decision gives a destination gap of 6.28–26.24×. This headroom justifies a **candidate ticket**, not a keep/adoption decision and not a claim to close that gap. Even the CP-only saturated diagnostic scale exceeds the prior below-floor scale, but the uninstrumented wall denominator and real optimization transfer remain unmeasured. A candidate still needs pristine-Rust value exactness, fresh both-leg output qualification and the predeclared paired ordinary + accelerated wall/CPU gate. C++ is a headroom reference, NOT a replacement proposal.",
        "",
        "## Evidence, validation and remaining gaps",
        "",
        "- `campaign.py`: sealed/resumable runner; raw manifest and initial/resume tee logs are under the gitignored scratch raw tree.",
        "- `analyze.py`: raw → portable reduced case medians and deterministic tables; `results/forward.json`, `results/reverse.json`, `results/summary.json`, `results/provenance.json`.",
        "- `verify.py`: checkout-only recomputation from committed results + canonical corpus and t60/t68 captures; checks report equality, source population/geometry, output invariance across passes and seal references; mutation controls reject changed timing, deleted case, shifted site and changed scope total.",
        "- Exact cross-engine hashes are order-sensitive, not production parity. RP closing/offset shapes missing; historical boolean time mix unknown; RP snapshots are not exhaustive frequency weights; CP is layer-sampled.",
        "- No production source, issue, map, engine or registry edits; no staging/commit; no slicer, cargo test or engine rebuild. No production speedup, CPU corroboration, or new wasm measurement. No internal-engine profile or confidence interval.",
        "- Verification outcome is recorded separately by the final verifier execution; this report is a deterministic rendering, so the verifier can reject numerical report drift.",
        "",
    ]
    return "\n".join(lines)


def main():
    RESULTS.mkdir(exist_ok=True)
    reduce_raw()
    summary = build_summary()
    save(RESULTS / "summary.json", summary)
    (HERE / "FINDINGS.md").write_text(render(summary), encoding="utf-8")
    print("reduced campaign and rendered FINDINGS.md")


if __name__ == "__main__":
    main()
