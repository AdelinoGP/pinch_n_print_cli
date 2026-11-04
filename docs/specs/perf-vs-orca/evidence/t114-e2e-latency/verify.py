#!/usr/bin/env python3
"""t114 e2e bucket latency take -- verifier.

Re-derives every headline number in FINDINGS.md from the committed
`summary.json`, and checks the two structural claims the take rests on
against the checkout:

  * the live loader still compiles each on-disk component serially in the
    binding loop and has no cache on the path;
  * the gated entry point still owns the `pnp_cli` freshness preflight.

  python verify.py              # verify the committed set (checkout-only)
  python verify.py --self-test  # negative controls: mutate the set, demand rejection

Wall samples are wall-clock, so the assertions are relationships (medians,
arithmetic, ordering) with absolute pins no tighter than 0.05 s.
"""

from __future__ import annotations

import copy
import json
import re
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
LOADER = REPO / "crates" / "slicer-wasm-host" / "src" / "execution_plan_live.rs"
XTASK_TEST = REPO / "xtask" / "src" / "test.rs"
SLICE_E2E = (
    REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "slice_end_to_end_tdd.rs"
)

EXPECTED_MODULE_SUM_S = 854.3
EXPECTED_TESTS = 178
EXPECTED_PLAIN_MEDIAN_S = 11.732
EXPECTED_PARALLEL_MEDIAN_S = 9.301
EXPECTED_PLAIN_FIXED_MS = 10011
EXPECTED_PARALLEL_FIXED_MS = 7168
EXPECTED_DELTA_PCT = 20.7
EXPECTED_FLOOR_S = 4065.3
EXPECTED_EXTRAPOLATED_S = 5263.1
EXPECTED_CACHE_S = 273.3
EXPECTED_INFLATION = 8.63
EXPECTED_STALE_FAILURES = 9


def load() -> dict:
    return json.loads((HERE / "summary.json").read_text(encoding="utf-8"))


def read_tree() -> dict:
    return {
        "loader": LOADER.read_text(encoding="utf-8"),
        "xtask_test": XTASK_TEST.read_text(encoding="utf-8"),
        "slice_e2e": SLICE_E2E.read_text(encoding="utf-8"),
        "cube_painted": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "cube_painted_e2e_tdd.rs").read_text(encoding="utf-8"),
        "cube_overrides": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "cube_painted_overrides_e2e_tdd.rs").read_text(encoding="utf-8"),
        "scenario": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "scenario_traces_tdd.rs").read_text(encoding="utf-8"),
        "threemf": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "threemf_fixture_e2e_tdd.rs").read_text(encoding="utf-8"),
        "modifier_infill": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "modifier_infill_tdd.rs").read_text(encoding="utf-8"),
        "precision": (REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "slicing_precision_integration_tdd.rs").read_text(encoding="utf-8"),
        "ir_tests": (REPO / "crates" / "slicer-ir" / "tests" / "ir_tests.rs").read_text(encoding="utf-8"),
    }


def median_of(samples: list[float]) -> float:
    return statistics.median(samples)


def check(b: dict, tree: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    s = b["slice"]

    # --- one slice, decomposed -------------------------------------------
    plain_med = median_of(s["plain_wall_s"])
    par_med = median_of(s["parallel_wall_s"])
    add(
        "plain slice median reproduces 11.732 s",
        abs(plain_med - EXPECTED_PLAIN_MEDIAN_S) <= 0.01,
        f"{plain_med:.3f}",
    )
    add(
        "parallel-probe median reproduces 9.301 s",
        abs(par_med - EXPECTED_PARALLEL_MEDIAN_S) <= 0.01,
        f"{par_med:.3f}",
    )
    add(
        "the probe is faster than HEAD",
        par_med < plain_med,
        f"{par_med:.3f} < {plain_med:.3f}",
    )

    for arm, expected in (
        ("plain_instrumented", EXPECTED_PLAIN_FIXED_MS),
        ("parallel_instrumented", EXPECTED_PARALLEL_FIXED_MS),
    ):
        run = s[arm]
        phases = sum(run["phases_ms"].values())
        fixed = run["slice_complete_ms"] - phases
        add(
            f"{arm}: phase sum is the measured 1,833 / 1,816 split",
            (phases == 1833) if arm == "plain_instrumented" else (phases == 1816),
            f"phases={phases}",
        )
        add(
            f"{arm}: fixed cost is slice_complete minus phases",
            abs(fixed - expected) <= 2,
            f"{fixed} ms vs {expected} ms",
        )
        add(
            f"{arm}: wall and slice_complete agree within 100 ms",
            abs(run["wall_s"] * 1000 - run["slice_complete_ms"]) <= 100,
            f"{run['wall_s'] * 1000:.0f} vs {run['slice_complete_ms']}",
        )

    add(
        "both arms emit byte-identical G-code",
        s["gcode_sha256"] == s["gcode_sha256_parallel"],
        f"{s['gcode_sha256'][:12]} vs {s['gcode_sha256_parallel'][:12]}",
    )
    delta = plain_med - par_med
    add(
        "median delta reproduces -2.431 s",
        abs(delta - s["delta_median_s"]) <= 0.01,
        f"{delta:.3f} vs {s['delta_median_s']}",
    )
    pct = delta / plain_med * 100.0
    add(
        "delta percentage reproduces 20.7 %",
        abs(pct - s["delta_median_pct"]) <= 0.1,
        f"{pct:.1f} vs {s['delta_median_pct']}",
    )

    # --- inventory arithmetic --------------------------------------------
    walls = [m["wall_s"] for m in b["modules"]]
    tests = [m["tests"] for m in b["modules"]]
    remainder = sum(walls)
    add(
        "the non-whale modules sum to 854.3 s",
        abs(remainder - EXPECTED_MODULE_SUM_S) <= 0.5,
        f"{remainder:.2f}",
    )
    w = b["whales"]
    total_tests = (
        sum(tests)
        + w["lightning_migration_tdd"]["tests_total"]
        + w["preparation_runtime_lifecycle_tdd"]["tests_total"]
    )
    add(
        "the inventory covers all 178 e2e tests",
        total_tests == EXPECTED_TESTS,
        f"{total_tests}",
    )
    floor = (
        remainder
        + w["lightning_migration_tdd"]["measured_wall_s"]
        + w["preparation_runtime_lifecycle_tdd"]["recorded_window_s"]
    )
    add(
        "the measured/cited floor reproduces 4,065 s",
        abs(floor - EXPECTED_FLOOR_S) <= 1.0,
        f"{floor:.1f}",
    )
    lightning_x = (
        w["lightning_migration_tdd"]["measured_wall_s"]
        * w["lightning_migration_tdd"]["tests_total"]
        / (
            w["lightning_migration_tdd"]["tests_completed"]
            + w["lightning_migration_tdd"]["tests_in_flight"]
        )
    )
    prep_x = (
        w["preparation_runtime_lifecycle_tdd"]["recorded_window_s"]
        * w["preparation_runtime_lifecycle_tdd"]["tests_total"]
        / w["preparation_runtime_lifecycle_tdd"]["recorded_test_count"]
    )
    extrapolated = remainder + lightning_x + prep_x
    add(
        "the extrapolated suite reproduces 5,263 s",
        abs(extrapolated - EXPECTED_EXTRAPOLATED_S) <= 1.0,
        f"{extrapolated:.1f} (lightning {lightning_x:.1f} + prep {prep_x:.1f} + rest {remainder:.1f})",
    )
    add(
        "the two whales are at least 79 % of the extrapolated suite",
        (lightning_x + prep_x) / extrapolated >= 0.79,
        f"{(lightning_x + prep_x) / extrapolated * 100:.1f} %",
    )
    add(
        "slice_end_to_end's 42 cached tests cost far less per test than a two-slice module",
        next(m["wall_s"] for m in b["modules"] if m["name"] == "slice_end_to_end_tdd")
        / 42.0
        < next(
            m["wall_s"]
            for m in b["modules"]
            if m["name"] == "calicat_internal_bridge_gating_e2e_tdd"
        )
        / 2.0,
        "cached-run module vs two-slice module",
    )
    e2e_src = tree["slice_e2e"]
    cached_sites = len(re.findall(r"cached_run\(", e2e_src))
    keys = set(re.findall(r"ModuleDirKind::([A-Za-z]+)", e2e_src))
    uncached = len(re.findall(r"run_pnp_cli_uncached", e2e_src))
    add(
        "slice_end_to_end collapses 42 tests into ~10 pnp_cli runs",
        cached_sites == 23
        and keys == {"CoreModules", "PartCoolingFiltered", "TreeSupportFiltered"}
        and uncached == 7,
        f"cached_sites={cached_sites} keys={len(keys)} uncached={uncached}",
    )

    # --- concurrency ------------------------------------------------------
    c = b["concurrency"]
    inflation = c["nextest_j12_test_s"] / c["alone_test_s"]
    add(
        "the concurrency inflation reproduces 8.63x",
        abs(inflation - EXPECTED_INFLATION) <= 0.01,
        f"{inflation:.2f}x",
    )
    add(
        "the concurrency arm is disclosed as an upper bound",
        b["load_disclosure"]["concurrency_factor_is_upper_bound"] is True,
        "load disclosure",
    )
    add(
        "twelve processes were observed concurrently",
        c["concurrent_processes_observed"] == 12,
        f"{c['concurrent_processes_observed']}",
    )

    # --- stale-binary gate ------------------------------------------------
    stale = b["stale_pnp_cli"]
    add(
        "nine tests failed on the stale-binary gate",
        stale["failed_tests"] == EXPECTED_STALE_FAILURES,
        f"{stale['failed_tests']}",
    )
    add(
        "the failure is a fast failure, not a slow one",
        stale["failure_time_s"] < 0.5,
        f"{stale['failure_time_s']} s",
    )

    # --- cache ceiling ----------------------------------------------------
    cc = b["cache_ceiling"]
    modelled = (
        cc["first_load_s"]
        + cc["slice_calls_estimate"] * cc["pipeline_s_per_cached_slice"]
    )
    add(
        "the cache ceiling reproduces 273.3 s",
        abs(modelled - EXPECTED_CACHE_S) <= 1.0,
        f"{modelled:.1f}",
    )
    add(
        "the cache ceiling is under the ticket's 10-minute gate",
        modelled < 600.0,
        f"{modelled / 60.0:.2f} min",
    )

    # --- adversarial audit (post-resolution) -----------------------------
    pa = b["post_audit"]
    add(
        "the post-audit e2e suite lists 173 tests",
        pa["e2e_list_count_after"] == 173,
        f"{pa['e2e_list_count_after']}",
    )
    mi = pa["modifier_infill"]
    mi_delta = mi["before_s"] - mi["after_s"]
    add(
        "the modifier_infill merge reproduces -11.98 s / -30.9 %",
        abs(mi_delta - 11.98) <= 0.02 and abs(mi_delta / mi["before_s"] * 100.0 - 30.9) <= 0.2,
        f"{mi_delta:.2f} s / {mi_delta / mi['before_s'] * 100.0:.1f} %",
    )
    prec = pa["slicing_precision"]
    add(
        "the precision merge is recorded as below-noise, not claimed",
        prec["claimed"] is False and prec["after_s"] >= prec["before_s"] - 1.0,
        f"{prec['before_s']} -> {prec['after_s']} s",
    )
    starts = pa["pipeline_starts_estimate"]
    load_s = pa["pipeline_start_load_s"]
    add(
        "237 pipeline starts at 10.0 s reproduce ~39.5 min of pure loading",
        abs(starts * load_s / 60.0 - 39.5) <= 0.5,
        f"{starts * load_s / 60.0:.1f} min",
    )
    add(
        "the audit's ~15 min duplicate work is second-order next to the load floor",
        pa["duplicate_work_estimate_s"] < starts * load_s,
        f"{pa['duplicate_work_estimate_s']} s duplicate vs {starts * load_s:.0f} s loading",
    )

    # --- audit edits present in the tree ---------------------------------
    add(
        "the decorative fixture test is gone",
        "painted_3mf_fixture_is_committed" not in tree["cube_painted"],
        "cube_painted",
    )
    add(
        "both test-local scenario_2 fakes are gone",
        "scenario_2_higher_paint_order_wins_for_custom_overlap" not in tree["scenario"]
        and "scenario_2_equal_paint_order_conflicting_values_are_fatal" not in tree["scenario"],
        "scenario_traces",
    )
    add(
        "the real PaintSemantic order claim moved to slicer-ir",
        "paint_semantic_derived_ordering_ranks_material_first" in tree["ir_tests"],
        "ir_tests",
    )
    add(
        "the lying override test name is replaced",
        "paint_config_override_parses_cleanly_on_the_live_path" in tree["cube_overrides"]
        and "paint_config_override_visibly_differs_gcode" not in tree["cube_overrides"],
        "cube_painted_overrides",
    )
    add(
        "the dead disk support-blocker test probes the real fixture",
        "bridge_support_enforcers.3mf" in tree["threemf"],
        "threemf_fixture",
    )
    add(
        "the boundary-anchoring assertions share the two-densities slice",
        "fn assert_boundary_anchoring(gcode: &str)" in tree["modifier_infill"]
        and "assert_boundary_anchoring(&gcode);" in tree["modifier_infill"]
        and "fn modifier_infill_boundary_anchoring" not in tree["modifier_infill"],
        "modifier_infill",
    )
    add(
        "the precision structural invariants run on the determinism output",
        "fn assert_legacy_structural_invariants(legacy_bytes: &[u8])" in tree["precision"]
        and "assert_legacy_structural_invariants(&a);" in tree["precision"]
        and "fn legacy_slice_structural_invariants" not in tree["precision"],
        "slicing_precision",
    )
    add(
        "the real-pipeline test no longer tolerates empty G-code",
        "!gcode.is_empty()" in tree["slice_e2e"],
        "slice_end_to_end",
    )

    # --- structural claims against the checkout --------------------------
    loader = tree["loader"]
    add(
        "the live loader compiles each module inside the binding loop",
        "for module in report.modules {" in loader
        and "compile_module_component(engine.as_ref(), &module)?" in loader,
        "loader loop",
    )
    add(
        "the live loader has no component cache on the path",
        "COMPONENT_CACHE" not in loader and "component_cache" not in loader,
        "no cache global",
    )
    add(
        "the 24-thread probe is not in the committed tree",
        "PROBE(t114)" not in loader,
        "probe reverted",
    )
    add(
        "the gated entry point owns the pnp_cli freshness preflight",
        "ensure_pnp_cli_fresh" in tree["xtask_test"],
        "xtask test preflight",
    )

    return out


def main() -> int:
    b = load()
    tree = read_tree()

    results = check(b, tree)
    failures = [r for r in results if not r[1]]
    for name, ok, detail in results:
        print(f"[{'ok' if ok else 'FAIL'}] {name}" + (f" — {detail}" if detail else ""))
    print()
    print(f"{len(results) - len(failures)}/{len(results)} checks passed")
    if failures:
        print(f"VERDICT: FAIL ({len(failures)} failing)")
        return 1

    if "--self-test" in sys.argv:
        controls: list[tuple[str, dict, dict]] = []

        def mutate_summary(label: str, fn) -> None:
            m = copy.deepcopy(b)
            fn(m)
            controls.append((label, m, tree))

        def mutate_tree(label: str, fn) -> None:
            t = copy.deepcopy(tree)
            fn(t)
            controls.append((label, b, t))

        mutate_summary(
            "plain median shifts by 0.2 s",
            lambda m: m["slice"]["plain_wall_s"].__setitem__(1, 12.0),
        )
        mutate_summary(
            "phase sum loses the per-layer phase",
            lambda m: m["slice"]["plain_instrumented"]["phases_ms"].__setitem__(
                "per_layer", 1170
            ),
        )
        mutate_summary(
            "G-code hashes diverge",
            lambda m: m["slice"].__setitem__("gcode_sha256_parallel", "0" * 64),
        )
        mutate_summary(
            "a module test count is dropped",
            lambda m: m["modules"][0].__setitem__("tests", 9),
        )
        mutate_summary(
            "the floor cites a smaller lightning window",
            lambda m: m["whales"]["lightning_migration_tdd"].__setitem__(
                "measured_wall_s", 1200
            ),
        )
        mutate_summary(
            "the concurrency factor is halved",
            lambda m: m["concurrency"].__setitem__("nextest_j12_test_s", 100.311),
        )
        mutate_summary(
            "the stale-binary count is zeroed",
            lambda m: m["stale_pnp_cli"].__setitem__("failed_tests", 0),
        )
        mutate_summary(
            "the cache ceiling grows past the gate",
            lambda m: m["cache_ceiling"].__setitem__("slice_calls_estimate", 400),
        )
        mutate_tree(
            "a component cache appears in the loader",
            lambda t: t.__setitem__(
                "loader", t["loader"] + "\nstatic COMPONENT_CACHE: () = ();\n"
            ),
        )
        mutate_tree(
            "the probe is left in the loader",
            lambda t: t.__setitem__("loader", t["loader"] + "\n// PROBE(t114)\n"),
        )
        mutate_tree(
            "the xtask freshness preflight is removed",
            lambda t: t.__setitem__(
                "xtask_test", t["xtask_test"].replace("ensure_pnp_cli_fresh", "removed")
            ),
        )
        mutate_tree(
            "a fourth module-dir key appears in slice_end_to_end",
            lambda t: t.__setitem__(
                "slice_e2e", t["slice_e2e"] + "\n// ModuleDirKind::Empty\n"
            ),
        )
        mutate_summary(
            "the modifier_infill delta is inflated",
            lambda m: m["post_audit"]["modifier_infill"].__setitem__("after_s", 18.0),
        )
        mutate_summary(
            "the precision merge is claimed despite being below noise",
            lambda m: m["post_audit"]["slicing_precision"].__setitem__("claimed", True),
        )
        mutate_summary(
            "the post-audit suite count is wrong",
            lambda m: m["post_audit"].__setitem__("e2e_list_count_after", 178),
        )
        mutate_tree(
            "the revived support-blocker candidate is removed",
            lambda t: t.__setitem__("threemf", t["threemf"].replace("bridge_support_enforcers.3mf", "removed.3mf")),
        )
        mutate_tree(
            "a deleted scenario_2 fake is re-added",
            lambda t: t.__setitem__(
                "scenario",
                t["scenario"]
                + chr(10)
                + "fn scenario_2_equal_paint_order_conflicting_values_are_fatal() {}"
                + chr(10),
            ),
        )

        missed = 0
        for label, m, t in controls:
            control_results = check(m, t)
            control_failures = [r for r in control_results if not r[1]]
            detected = bool(control_failures)
            print(f"[{'rejected' if detected else 'MISSED'}] negative control: {label}")
            if not detected:
                missed += 1
        print()
        print(
            f"self-test: {len(controls) - missed}/{len(controls)} negative controls detected"
        )
        if missed:
            print("VERDICT: FAIL (self-test missed a control)")
            return 1

    print("VERDICT: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
