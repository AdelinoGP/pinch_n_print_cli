"""Re-read the completed diagnostic pair; never launches a slicer or a build."""

from collections import Counter
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "t43_diagnostics", HERE / "run_diagnostics.py"
)
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
require = helper.require
state = helper.read_json(HERE / "attempt-1/results.json")
require(
    state["status"] == "completed-diagnostics-not-acceptance"
    and state["slices_launched"] == 2
    and set(state["runs"]) == {"ordinary", "accelerated"},
    "incomplete diagnostic pair",
)
require(
    helper.digest(HERE / "run_diagnostics.py") == state["helper_sha256"],
    "runner source changed",
)
require(
    [c["name"] for c in state["commands"]]
    == [
        "reference-freeze-before",
        "slice-ordinary",
        "slice-accelerated",
        "reference-freeze-after",
    ]
    and all(c["exit_code"] == 0 for c in state["commands"]),
    "command sequence/status",
)
require(
    "identity_before" in state and "identity_after" in state, "missing identity gates"
)
raw = Path(state["durable_raw_root"])
require(
    helper.read_json(raw / "results.json") == state, "durable provenance copy differs"
)
for name, expected in state["raw_inventory_sha256"].items():
    require(helper.digest(raw / name) == expected, f"raw diagnostic changed: {name}")

rows = []
for mode, run in state["runs"].items():
    trace = Path(run["trace_path"])
    require(helper.digest(trace) == run["trace_sha256"], f"trace changed: {mode}")
    require(
        helper.reduce_events(trace) == run["reduction"],
        f"event reduction differs: {mode}",
    )
    with trace.open(encoding="utf-8") as stream:
        events = [json.loads(line) for line in stream if line.lstrip().startswith("{")]
    for prefix in ("stage", "module"):

        def keys(edge):
            return Counter(
                (
                    e.get("phase"),
                    e.get("stage"),
                    e.get("layer_index"),
                    e.get("module_id"),
                )
                for e in events
                if e["event"] == f"{prefix}_{edge}"
            )

        require(keys("start") == keys("complete"), f"unmatched {prefix} events: {mode}")
    layers = [e for e in events if e["event"] == "layer_complete"]
    count = next(
        e["layer_count"]
        for e in events
        if e["event"] == "phase_start" and e["phase"] == "per_layer"
    )
    require(
        len(layers) == count
        and {e["layer_index"] for e in layers} == set(range(count)),
        f"layer coverage: {mode}",
    )
    require(
        {
            e["layer_index"]
            for e in sorted(layers, key=lambda e: e["elapsed_ms"], reverse=True)[:2]
        }
        == {1, 2},
        f"longest layer pair changed: {mode}",
    )
    for index, detail in run["reduction"]["tail_layers"].items():
        top = detail["modules_by_elapsed"][0]
        rows.append(
            {
                "mode": mode,
                "layer_index": int(index),
                "layer_elapsed_ms": detail["elapsed_ms"],
                "top_module": top["module_id"],
                "top_module_elapsed_ms": top["elapsed_ms"],
                "share_of_layer_interval_percent": 100
                * top["elapsed_ms"]
                / detail["elapsed_ms"],
            }
        )

original = helper.read_json(helper.CAMPAIGN / "hash-inventory.json")
original_counts = {}
for root, key in (
    (
        Path(original["durable_raw_archive"]),
        "durable_raw_archive_files_excluding_its_inventory",
    ),
    (helper.CAMPAIGN, "tracked_evidence_files_excluding_this_inventory"),
):
    for name, expected in original[key].items():
        require(
            helper.digest(root / name) == expected, f"original campaign changed: {name}"
        )
    original_counts[key] = len(original[key])

manifest = helper.read_json(helper.SNAPSHOTS)
linker_key = "modules/infill-linker/infill-linker.wasm"
linker_hashes = {}
for mode in ("ordinary", "accelerated"):
    frozen = manifest[f"{mode}_snapshot"]
    actual = helper.digest(Path(frozen["root"]) / linker_key)
    require(
        actual == frozen["inventory_sha256"][linker_key], "linker artifact identity"
    )
    linker_hashes[mode] = actual

result = {
    "status": "passed",
    "kind": "diagnostic-evidence-verification-not-acceptance",
    "recomputed_event_reductions_match": True,
    "stage_module_event_pairs_complete": True,
    "layer_coverage_complete": True,
    "original_campaign_files_verified": original_counts,
    "infill_linker_artifact_sha256": linker_hashes,
    "infill_linker_artifacts_byte_identical": len(set(linker_hashes.values())) == 1,
    "tail_owner_rows": rows,
}
destination = HERE / "attempt-1/verification.json"
if destination.exists():
    require(
        helper.read_json(destination) == result,
        "saved verification differs from recheck",
    )
else:
    with destination.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
