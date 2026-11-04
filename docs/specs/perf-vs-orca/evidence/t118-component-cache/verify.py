#!/usr/bin/env python3
"""t118 compiled-component cache -- verifier.

Re-derives the t118 headline numbers from `summary.json` and checks the tree
for the three levers:

  * a process-global compiled-component cache and its use by the live loader;
  * wasmtime's validated on-disk compile cache;
  * rayon parallelism in the per-layer capture path and in the lightning corpus.

  python verify.py              # verify the committed set (checkout-only)
  python verify.py --self-test  # negative controls: mutate the set, demand rejection
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]

CACHE_SRC = REPO / "crates" / "slicer-wasm-host" / "src" / "component_cache.rs"
LOADER = REPO / "crates" / "slicer-wasm-host" / "src" / "execution_plan_live.rs"
INSTANCE = REPO / "crates" / "slicer-wasm-host" / "src" / "instance.rs"
LAYER_EXEC = REPO / "crates" / "slicer-runtime" / "src" / "layer_executor.rs"
LIGHTNING = (
    REPO / "crates" / "slicer-runtime" / "tests" / "e2e" / "lightning_migration_tdd.rs"
)
CARGO = REPO / "Cargo.toml"
GCODE_SHA = "d12cdfd10dab66a0010a0b5d5fca53d9387f4e96227ad5e74d0ec18197c0e260"
LIMIT_S = 900.0


def load() -> dict:
    return json.loads((HERE / "summary.json").read_text(encoding="utf-8"))


def read_tree() -> dict:
    return {
        "cache": CACHE_SRC.read_text(encoding="utf-8"),
        "loader": LOADER.read_text(encoding="utf-8"),
        "instance": INSTANCE.read_text(encoding="utf-8"),
        "layer_exec": LAYER_EXEC.read_text(encoding="utf-8"),
        "lightning": LIGHTNING.read_text(encoding="utf-8"),
        "cargo": CARGO.read_text(encoding="utf-8"),
    }


def check(b: dict, tree: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    s = b["slice"]
    add(
        "the warm slice is faster than the pre-cache median",
        s["warm_median_s"] < s["before_median_s"],
        f"{s['warm_median_s']} < {s['before_median_s']}",
    )
    add(
        "the cold run is slower than the warm run (cache is doing work)",
        s["cold_s"] > s["warm_median_s"],
        f"cold {s['cold_s']} > warm {s['warm_median_s']}",
    )
    add(
        "the G-code hash is the unchanged calicat reference",
        s["gcode_sha256"] == GCODE_SHA,
        s["gcode_sha256"][:12],
    )

    for m in b["modules"]:
        add(
            f"{m['name']}: after is faster than before",
            m["after_s"] < m["before_s"],
            f"{m['before_s']} -> {m['after_s']}",
        )

    fs = b["full_suite"]
    add(
        "the full bucket clears the 15-minute milestone",
        fs["after_case_parallel_s"] < LIMIT_S,
        f"{fs['after_case_parallel_s']} s < {LIMIT_S} s",
    )
    add(
        "the milestone headroom reproduces",
        abs((LIMIT_S - fs["after_case_parallel_s"]) - fs["headroom_s"]) <= 0.1,
        f"{LIMIT_S - fs['after_case_parallel_s']:.2f} s",
    )
    add(
        "case parallelism improved on the caches-and-layers run",
        fs["after_case_parallel_s"] < fs["after_caches_and_layers_s"],
        f"{fs['after_caches_and_layers_s']} -> {fs['after_case_parallel_s']}",
    )
    add(
        "the full bucket is faster than the t114 measured floor",
        fs["after_case_parallel_s"] < fs["before_floor_s"],
        f"{fs['after_case_parallel_s']} < {fs['before_floor_s']}",
    )
    add(
        "the suite is 172 passed + 1 ignored",
        fs["tests"] == 172 and fs["ignored"] == 1,
        f"{fs['tests']} + {fs['ignored']}",
    )
    add(
        "the disk cache stays under 100 MB",
        b["cache"]["disk_size_mb"] <= 100,
        f"{b['cache']['disk_size_mb']} MB",
    )

    # --- tree checks ------------------------------------------------------
    cache = tree["cache"]
    add(
        "the process cache keys on profile + path + mtime + length",
        all(
            token in cache
            for token in (
                "profile: bool",
                "path: PathBuf",
                "modified: SystemTime",
                "len: u64",
            )
        ),
        "component_cache key",
    )
    add(
        "the process cache never caches failures",
        "Compilation failures are never cached." in cache
        and "compile_component(bytes)?" in cache
        and cache.count(".insert(key, Arc::clone(&component));") == 1,
        "error path",
    )
    add(
        "the live loader uses the shared engine and the cached component",
        "component_cache::shared_engine(profile)" in tree["loader"]
        and "compile_module_component(profile, &module)" in tree["loader"],
        "loader wiring",
    )
    add(
        "the engine enables wasmtime's disk cache",
        "apply_disk_compile_cache(&mut config)" in tree["instance"]
        and "config.cache(Some(cache))" in tree["instance"],
        "disk cache",
    )
    add(
        "the wasmtime dependency enables the cache feature",
        'features = ["call-hook", "cache"]' in tree["cargo"],
        "Cargo.toml",
    )
    layer = tree["layer_exec"]
    capture_fn = layer.split("pub fn execute_captured_stages_with_support_tools", 1)[-1]
    add(
        "the capture path uses rayon layer parallelism",
        ".par_iter()" in capture_fn.split("pub fn execute_blackboard_taps", 1)[0],
        "layer_executor capture",
    )
    add(
        "the lightning corpus tests parallelize their independent cases",
        tree["lightning"].count("baseline.cases.par_iter().for_each(|case| {") >= 2,
        "lightning cases",
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
            "the full bucket slips past 15 minutes",
            lambda m: m["full_suite"].__setitem__("after_case_parallel_s", 905.0),
        )
        mutate_summary(
            "the warm slice is slower than before",
            lambda m: m["slice"].__setitem__("warm_median_s", 12.0),
        )
        mutate_summary(
            "the G-code hash changes",
            lambda m: m["slice"].__setitem__("gcode_sha256", "0" * 64),
        )
        mutate_summary(
            "a module regresses",
            lambda m: m["modules"][0].__setitem__("after_s", 2000.0),
        )
        mutate_summary(
            "the headroom is miscomputed",
            lambda m: m["full_suite"].__setitem__("headroom_s", 0.0),
        )
        mutate_tree(
            "the process cache stops keying on mtime",
            lambda t: t.__setitem__(
                "cache", t["cache"].replace("modified: SystemTime,", "modified: (),")
            ),
        )
        mutate_tree(
            "the live loader bypasses the shared engine",
            lambda t: t.__setitem__(
                "loader",
                t["loader"].replace(
                    "component_cache::shared_engine(profile)",
                    "Arc::new(WasmEngine::with_profiling(profile))",
                ),
            ),
        )
        mutate_tree(
            "the disk cache is removed",
            lambda t: t.__setitem__(
                "instance",
                t["instance"].replace("apply_disk_compile_cache(&mut config);", ""),
            ),
        )
        mutate_tree(
            "the cache Cargo feature is removed",
            lambda t: t.__setitem__(
                "cargo",
                t["cargo"].replace(
                    'features = ["call-hook", "cache"]', 'features = ["call-hook"]'
                ),
            ),
        )
        mutate_tree(
            "the capture loop goes back to serial",
            lambda t: t.__setitem__(
                "layer_exec",
                t["layer_exec"].replace(
                    "sorted_layers\n        .par_iter()",
                    "sorted_layers\n        .iter()",
                    1,
                ),
            ),
        )
        mutate_tree(
            "the lightning cases go back to serial",
            lambda t: t.__setitem__(
                "lightning",
                t["lightning"].replace(
                    "baseline.cases.par_iter().for_each(|case| {",
                    "for case in &baseline.cases {",
                    2,
                ),
            ),
        )

        missed = 0
        for label, m, t in controls:
            control_failures = [r for r in check(m, t) if not r[1]]
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
