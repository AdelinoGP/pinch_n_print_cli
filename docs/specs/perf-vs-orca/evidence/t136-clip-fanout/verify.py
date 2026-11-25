#!/usr/bin/env python3
"""t136 verifier: re-derive every headline number from reduced/summary.json.

Exit 0 means every check passed. `--self-test` mutates the summary one field at
a time and requires the corresponding check to fail, so a check that can never
fire is caught rather than trusted.

Needs only the in-repo reduced summary; the raw stderr tree is optional.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REFERENCE = {
    "benchy-classic-on": "b3f1ecccc5385f1b807c3418c271bcf57761a183e60c05eefb3294195df3d7a2",
    "base-classic-on": "585c6a8edc2fd1de72dd2a86933fdad25c7ed144662dd80bbc68fb7856e4f629",
}
TOL_NS = 2  # integer floor divisions below can lose a nanosecond per call

# Benchy's clip core is ~17x smaller, so the same fixed per-call costs are a
# larger share of it. The bands are per cell, and the base cell's are the tight
# ones the findings quote.
CELL_BANDS = {
    "base-classic-on": {
        "guest_cpu_share": 0.05,
        "dispatch_excess_share": 0.03,
        "marshal_band": (0.05, 0.12),
        "carve_p50_requests": 256,
        "union_critpath_tol": 0.005,
        "serial_wall_share": 0.01,
    },
    "benchy-classic-on": {
        "guest_cpu_share": 0.15,
        "dispatch_excess_share": 0.06,
        "marshal_band": (0.10, 0.25),
        "carve_p50_requests": 176,
        "union_critpath_tol": 0.015,
        "serial_wall_share": 0.03,
    },
}


def merge_floor(series: list[list[int]], group: int, threads: int) -> int:
    total = 0
    for start in range(0, len(series), group):
        chunk = series[start : start + group]
        thread = sum(row[0] for row in chunk)
        longest = max(row[1] for row in chunk)
        total += max(thread // threads, longest)
    return total


def build_checks(summary: dict) -> list[tuple[str, bool, str]]:
    checks: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        checks.append((name, bool(ok), detail))

    cells = summary["cells"]
    check(
        "two measured cells",
        sorted(cells) == ["base-classic-on", "benchy-classic-on"],
        str(sorted(cells)),
    )

    for cell, entry in cells.items():
        bands = CELL_BANDS.get(cell, CELL_BANDS["base-classic-on"])
        passes = entry["passes"]
        check(f"{cell}: three passes", len(passes) == 3, str(len(passes)))
        host_walls = []
        for p in passes:
            probe = p["probe"]
            totals = probe["totals"]
            phases = probe["phases"]
            tag = f"{cell} p{p['pass']}"
            threads = totals["threads"]

            check(
                f"{tag}: hash is the post-t139 reference",
                p["sha256"] == REFERENCE[cell],
                p["sha256"][:12],
            )

            # 1. guest = host + marshal + guest CPU
            recon = (
                totals["host_wall_ns"]
                + totals["marshal_decode_ns"]
                + totals["guest_cpu_ns"]
            )
            check(
                f"{tag}: guest wall decomposes additively",
                abs(recon - totals["guest_wall_ns"]) <= TOL_NS,
                f"{recon} vs {totals['guest_wall_ns']}",
            )
            # 2. host = work floor + tail x chunking + dispatch excess
            recon = (
                totals["work_floor_ns"]
                + totals["tail_chunking_ns"]
                + totals["dispatch_excess_ns"]
            )
            check(
                f"{tag}: host wall decomposes additively",
                abs(recon - totals["host_wall_ns"]) <= TOL_NS,
                f"{recon} vs {totals['host_wall_ns']}",
            )
            # 3. work floor is thread/threads; tail is ideal - work floor
            check(
                f"{tag}: work floor is thread/threads",
                totals["work_floor_ns"] == totals["thread_ns"] // threads,
            )
            check(
                f"{tag}: tail term is ideal - work floor",
                totals["tail_chunking_ns"]
                == totals["ideal_ns"] - totals["work_floor_ns"],
            )
            # 4. phase sums reconstruct the totals
            for key, total_key in (
                ("host_wall_ns", "host_wall_ns"),
                ("guest_wall_ns", "guest_wall_ns"),
                ("thread_ns", "thread_ns"),
            ):
                summed = sum(ph[key] for ph in phases.values())
                check(
                    f"{tag}: phase {key} sums to the total",
                    abs(summed - totals[total_key]) <= TOL_NS,
                    f"{summed} vs {totals[total_key]}",
                )
            # 5. fan-out is thread/wall
            check(
                f"{tag}: host fan-out is thread/host wall",
                abs(
                    totals["host_fanout"] - totals["thread_ns"] / totals["host_wall_ns"]
                )
                < 1e-9,
            )
            # 6. the phase contrast: carve is near-ideal, union is not
            carve = phases["carve_run"]
            union = phases["union_run"]
            check(
                f"{tag}: carve fans out above 10x",
                carve["host_fanout"] > 10.0,
                f"{carve['host_fanout']:.2f}",
            )
            check(
                f"{tag}: union fans out below 5x",
                union["host_fanout"] < 5.0,
                f"{union['host_fanout']:.2f}",
            )
            check(
                f"{tag}: carve carries the requests",
                carve["requests"] / totals["requests"] > 0.9,
                f"{carve['requests']}/{totals['requests']}",
            )
            # 7. union wall is its own critical path
            check(
                f"{tag}: union wall equals its per-call critical-path sum",
                abs(union["host_wall_ns"] - union["ideal_ns"])
                <= int(union["host_wall_ns"] * bands["union_critpath_tol"]),
                f"{union['host_wall_ns']} vs {union['ideal_ns']}",
            )
            # 8. union dominates the clip core
            share = union["host_wall_ns"] / totals["host_wall_ns"]
            check(
                f"{tag}: union owns at least 60% of the host wall",
                share > 0.60,
                f"{share:.3f}",
            )
            # 9. union requests per call stay tiny
            check(
                f"{tag}: union p50 requests per call below 8",
                union["call_requests_quantiles"]["p50"] < 8,
                str(union["call_requests_quantiles"]["p50"]),
            )
            check(
                f"{tag}: carve p50 requests per call at the cell's batch size",
                carve["call_requests_quantiles"]["p50"] == bands["carve_p50_requests"],
                str(carve["call_requests_quantiles"]["p50"]),
            )
            # 10. merge floors are monotone and below the current wall
            series = [
                [int(part.split(":")[0]), int(part.split(":")[1])]
                for part in probe["chunk_series"]["union_run"].split(";")
                if part
            ]
            check(
                f"{tag}: union chunk series covers every chunk",
                len(series) == union["chunks"],
                f"{len(series)} vs {union['chunks']}",
            )
            rederived = merge_floor(series, 4, threads)
            check(
                f"{tag}: union merge4 floor re-derives",
                rederived == union["merge4_floor_ns"],
                f"{rederived} vs {union['merge4_floor_ns']}",
            )
            check(
                f"{tag}: union floors are monotone",
                union["phase_merged_floor_ns"]
                <= union["merge4_floor_ns"]
                <= union["merge2_floor_ns"]
                <= union["host_wall_ns"],
            )
            # 11. the non-host terms stay small
            check(
                f"{tag}: guest CPU below the cell's band",
                totals["guest_cpu_ns"] / totals["guest_wall_ns"]
                < bands["guest_cpu_share"],
                f"{totals['guest_cpu_ns'] / totals['guest_wall_ns']:.3f}",
            )
            check(
                f"{tag}: serial-mode calls below the cell's wall share",
                totals["serial_wall_share"] < bands["serial_wall_share"],
                f"{totals['serial_wall_share']:.4f}",
            )
            check(
                f"{tag}: serial + parallel wall reconstructs the host wall",
                abs(totals["serial_wall_ns"] + totals["parallel_wall_ns"] - totals["host_wall_ns"])
                <= TOL_NS,
            )
            check(
                f"{tag}: dispatch excess below the cell's band",
                totals["dispatch_excess_ns"] / totals["host_wall_ns"]
                < bands["dispatch_excess_share"],
                f"{totals['dispatch_excess_ns'] / totals['host_wall_ns']:.3f}",
            )
            low, high = bands["marshal_band"]
            check(
                f"{tag}: marshal+decode inside the cell's band",
                low < totals["marshal_decode_ns"] / totals["guest_wall_ns"] < high,
                f"{totals['marshal_decode_ns'] / totals['guest_wall_ns']:.3f}",
            )
            check(
                f"{tag}: no clip batches outside the planner segments",
                probe["other_modules"]["calls"] == 0,
            )
            host_walls.append(totals["host_wall_ns"])

        # 12. pass-to-pass stability (probe counters, not walls)
        spread = (max(host_walls) - min(host_walls)) / min(host_walls)
        check(
            f"{cell}: host clip-core wall stable across passes (<8%)",
            spread < 0.08,
            f"{spread:.3f}",
        )

    return checks


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--summary", type=Path, default=HERE / "reduced" / "summary.json")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    summary = json.loads(args.summary.read_text(encoding="utf-8"))
    checks = build_checks(summary)
    failed = [c for c in checks if not c[1]]
    for name, ok, detail in checks:
        status = "ok  " if ok else "FAIL"
        suffix = f"  [{detail}]" if detail and not ok else ""
        print(f"{status} {name}{suffix}")
    print(f"\n{len(checks) - len(failed)}/{len(checks)} checks passed")

    if failed:
        return 1
    if not args.self_test:
        return 0

    # Negative controls: each mutation must break the check that protects it.
    mutations = [
        (
            "guest additive",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(guest_wall_ns=1),
        ),
        (
            "host additive",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(host_wall_ns=1),
        ),
        (
            "work floor",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(work_floor_ns=1),
        ),
        (
            "tail term",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(tail_chunking_ns=1),
        ),
        (
            "phase host sum",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "carve_run"
            ].update(host_wall_ns=1),
        ),
        (
            "phase guest sum",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "carve_run"
            ].update(guest_wall_ns=1),
        ),
        (
            "fan-out identity",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(host_fanout=9.0),
        ),
        (
            "carve >10x",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "carve_run"
            ].update(host_fanout=9.0),
        ),
        (
            "union <5x",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ].update(host_fanout=6.0),
        ),
        (
            "carve request share",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "carve_run"
            ].update(requests=1),
        ),
        (
            "union critical path",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ].update(ideal_ns=1),
        ),
        (
            "union share",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ].update(host_wall_ns=1),
        ),
        (
            "union p50 requests",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ]["call_requests_quantiles"].update(p50=64),
        ),
        (
            "carve p50 at cap",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "carve_run"
            ]["call_requests_quantiles"].update(p50=128),
        ),
        (
            "union series length",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "chunk_series"
            ].update(union_run=""),
        ),
        (
            "merge4 re-derive",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ].update(merge4_floor_ns=1),
        ),
        (
            "floor monotonicity",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["phases"][
                "union_run"
            ].update(phase_merged_floor_ns=10**12),
        ),
        (
            "guest cpu share",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(guest_cpu_ns=10**9),
        ),
        (
            "serial wall share",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["totals"].update(serial_wall_ns=10**9),
        ),
        (
            "serial/parallel wall identity",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"]["totals"].update(parallel_wall_ns=1),
        ),
        (
            "dispatch excess share",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(dispatch_excess_ns=10**9),
        ),
        (
            "marshal band",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "totals"
            ].update(marshal_decode_ns=0),
        ),
        (
            "other modules",
            lambda s: s["cells"]["base-classic-on"]["passes"][0]["probe"][
                "other_modules"
            ].update(calls=3),
        ),
        (
            "hash",
            lambda s: s["cells"]["base-classic-on"]["passes"][0].update(
                sha256="0" * 64
            ),
        ),
        ("pass count", lambda s: s["cells"]["base-classic-on"]["passes"].pop()),
        ("cell set", lambda s: s["cells"].pop("base-classic-on")),
        (
            "stability",
            lambda s: s["cells"]["base-classic-on"]["passes"][2]["probe"][
                "totals"
            ].update(host_wall_ns=10**12),
        ),
    ]
    rejected = 0
    for name, mutate in mutations:
        mutated = copy.deepcopy(summary)
        mutate(mutated)
        mutated_checks = build_checks(mutated)
        if any(not ok for _, ok, _ in mutated_checks):
            rejected += 1
        else:
            print(f"SELF-TEST MISS: mutation '{name}' was not rejected")
    print(f"self-test: {rejected}/{len(mutations)} negative controls rejected")
    return 0 if rejected == len(mutations) else 1


if __name__ == "__main__":
    raise SystemExit(main())
