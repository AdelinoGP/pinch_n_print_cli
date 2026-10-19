"""One-shot, two-slice attribution only. No builds, profiles or acceptance runs."""

from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[5]
HERE = Path(__file__).resolve().parent
ATTEMPT = HERE / "attempt-1"
SNAPSHOTS = (
    ROOT
    / "docs/specs/perf-vs-orca/evidence/t38-adoption-resumed/snapshots/snapshot-manifest.json"
)
CAMPAIGN = (
    ROOT / "docs/specs/perf-vs-orca/evidence/t38-adoption-resumed/campaign/attempt-1"
)
REFERENCE = (
    ROOT / "docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/attempt-1"
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(root):
    require(root.is_dir(), f"missing frozen root: {root}")
    return {
        p.relative_to(root).as_posix(): digest(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def save():
    (ATTEMPT / "results.json").write_text(
        json.dumps(state, indent=2) + "\n", encoding="utf-8"
    )


def command(name, argv):
    entry = {
        "name": name,
        "argv": list(map(str, argv)),
        "cwd": str(ROOT),
        "exit_code": None,
    }
    state["commands"].append(entry)
    save()
    with (
        (ATTEMPT / f"{name}.stdout.log").open("xb") as out,
        (ATTEMPT / f"{name}.stderr.log").open("xb") as err,
    ):
        completed = subprocess.run(
            entry["argv"], cwd=ROOT, stdout=out, stderr=err, timeout=600
        )
    entry["exit_code"] = completed.returncode
    save()
    require(completed.returncode == 0, f"{name} exit {completed.returncode}")


def identities(phase):
    manifest = read_json(SNAPSHOTS)
    old_result = read_json(CAMPAIGN / "results.json")
    old_check = next(
        c["details"]
        for c in old_result["checks"]
        if c["name"] == "frozen-identities-before"
    )
    require(
        digest(SNAPSHOTS)
        == old_check["snapshot_manifest"]["expected_sha256"]
        == SNAPSHOTS.with_suffix(".sha256").read_text().strip(),
        "snapshot manifest identity",
    )
    durable_manifest = Path(manifest["durable_root"]) / "snapshot-manifest.json"
    require(
        durable_manifest.read_bytes() == SNAPSHOTS.read_bytes()
        and durable_manifest.with_suffix(".sha256").read_text().strip()
        == digest(SNAPSHOTS),
        "durable snapshot manifest identity",
    )
    require(
        digest(REFERENCE / "frozen-manifest.json")
        == manifest["t41_reference"]["manifest_sha256"],
        "reference producer manifest identity",
    )
    details = {
        "snapshot_manifest_sha256": digest(SNAPSHOTS),
        "snapshots": {},
        "campaign_inputs": {},
    }
    namespace = ROOT / ".local-artifacts/perimeter-reference-preparation"
    for mode in ("ordinary", "accelerated"):
        frozen = manifest[f"{mode}_snapshot"]
        root = Path(frozen["root"])
        require(
            root.resolve().is_relative_to(namespace.resolve()),
            "snapshot is not durable",
        )
        actual = inventory(root)
        require(
            actual == frozen["inventory_sha256"], f"{mode} snapshot identity changed"
        )
        details["snapshots"][mode] = {"root": str(root), "files_verified": len(actual)}
    for name, expected in manifest["campaign_inputs"].items():
        require(digest(ROOT / name) == expected, f"campaign input changed: {name}")
        details["campaign_inputs"][name] = expected
    original = read_json(CAMPAIGN / "hash-inventory.json")
    for root, key in (
        (
            Path(original["durable_raw_archive"]),
            "durable_raw_archive_files_excluding_its_inventory",
        ),
        (CAMPAIGN, "tracked_evidence_files_excluding_this_inventory"),
    ):
        for name, expected in original[key].items():
            require(
                digest(root / name) == expected,
                f"original campaign artifact changed: {name}",
            )
        details[key] = len(original[key])
    command(
        f"reference-freeze-{phase}",
        [sys.executable, REFERENCE.parent / "verify-freeze.py", REFERENCE],
    )
    state[f"identity_{phase}"] = details
    save()
    return manifest


def reduce_events(path):
    with path.open(encoding="utf-8") as stream:
        events = [json.loads(line) for line in stream if line.lstrip().startswith("{")]
    counts = Counter(e["event"] for e in events)
    require(
        counts["slice_complete"] == 1
        and counts["module_complete"] > 0
        and counts["stage_complete"] > 0
        and counts["profile_summary"] == 0,
        "missing instrumented events or unexpected profiling",
    )
    complete = next(e for e in events if e["event"] == "slice_complete")
    require(
        complete["status"] == "ok"
        and not complete["degraded"]
        and complete["fatal_error_count"] == complete["non_fatal_error_count"] == 0,
        "unclean slice completion",
    )
    layers = {}
    for index in (1, 2):
        ends = [
            e
            for e in events
            if e["event"] == "layer_complete" and e["layer_index"] == index
        ]
        require(len(ends) == 1, f"missing layer {index}")
        modules = [
            e
            for e in events
            if e["event"] == "module_complete" and e.get("layer_index") == index
        ]
        stages = [
            e
            for e in events
            if e["event"] == "stage_complete" and e.get("layer_index") == index
        ]
        layers[str(index)] = {
            "elapsed_ms": ends[0]["elapsed_ms"],
            "modules_by_elapsed": sorted(
                modules, key=lambda e: e["elapsed_ms"], reverse=True
            ),
            "stages_by_elapsed": sorted(
                stages, key=lambda e: e["elapsed_ms"], reverse=True
            ),
        }
    return {
        "event_counts": dict(counts),
        "completion": complete,
        "tail_layers": layers,
        "phase_completions": [e for e in events if e["event"] == "phase_complete"],
        "prepass_stages_by_elapsed": sorted(
            [
                e
                for e in events
                if e["event"] == "stage_complete" and e.get("phase") == "prepass"
            ],
            key=lambda e: e["elapsed_ms"],
            reverse=True,
        ),
        "prepass_modules_by_elapsed": sorted(
            [
                e
                for e in events
                if e["event"] == "module_complete" and e.get("phase") == "prepass"
            ],
            key=lambda e: e["elapsed_ms"],
            reverse=True,
        ),
    }


def main():
    global state
    require(not ATTEMPT.exists(), f"one-shot guard: {ATTEMPT} already exists; no retry")
    ATTEMPT.mkdir(parents=True)
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    raw = (
        ROOT
        / ".local-artifacts/perimeter-reference-preparation"
        / f"t43-diagnostics-{stamp}"
    )
    raw.mkdir()
    state = {
        "kind": "instrumented-attribution-not-acceptance",
        "status": "started",
        "authorization": "one ordinary plus one accelerated instrumented Arachne slice; no retries",
        "durable_raw_root": str(raw),
        "helper_sha256": digest(Path(__file__)),
        "commands": [],
        "slices_launched": 0,
        "runs": {},
    }
    save()
    failure = None
    try:
        overrides = [k for k in os.environ if k.upper().startswith(("PNP_", "SLICER_"))]
        require(not overrides, f"unscoped environment override names: {overrides}")
        manifest = identities("before")
        cell = Path(manifest["t41_reference"]["corpus_root"]) / "supports-off-benchy"
        reference_hash = digest(cell / "reference-arachne.gcode")
        for mode in ("ordinary", "accelerated"):
            frozen = manifest[f"{mode}_snapshot"]
            mode_dir = raw / mode
            mode_dir.mkdir()
            output = mode_dir / "output.gcode"
            trace = mode_dir / f"{mode}.jsonl"
            state["slices_launched"] += 1
            save()
            command(
                f"slice-{mode}",
                [
                    "pwsh",
                    "-NoProfile",
                    "-File",
                    ROOT / "resources/perimeter-acceptance/run_bench.ps1",
                    "-ExePath",
                    frozen["exe"],
                    "-InputModel",
                    cell / "model.stl",
                    "-Config",
                    cell / "arachne.json",
                    "-ModuleDir",
                    frozen["modules_dir"],
                    "-OutputPath",
                    output,
                    "-Threads",
                    "12",
                    "-Runs",
                    "1",
                    "-Label",
                    mode,
                    "-ResultsPath",
                    mode_dir / "diagnostic.csv",
                    "-Instrumented",
                    "-ExpectedGenerator",
                    "arachne",
                ],
            )
            # run_bench's real shared validator runs even though Instrumented suppresses timing rows.
            require(
                digest(output) == reference_hash,
                f"{mode} output differs from frozen Arachne reference",
            )
            reduction = reduce_events(trace)
            state["runs"][mode] = {
                "output_sha256": digest(output),
                "reference_byte_identical": True,
                "trace_sha256": digest(trace),
                "trace_path": str(trace),
                "reduction": reduction,
            }
            save()
    except Exception as error:
        failure = f"{type(error).__name__}: {error}"
        state["error"] = failure
    finally:
        try:
            identities("after")
        except Exception as error:
            state["post_integrity_error"] = f"{type(error).__name__}: {error}"
            failure = failure or state["post_integrity_error"]
        state["raw_inventory_sha256"] = inventory(raw)
        state["status"] = (
            "blocked" if failure else "completed-diagnostics-not-acceptance"
        )
        state["finished_utc"] = datetime.now(timezone.utc).isoformat()
        save()
        shutil.copyfile(ATTEMPT / "results.json", raw / "results.json")
    print(f"FACT {state['status']}; slices launched: {state['slices_launched']}")
    if failure:
        print(failure)
        return 1
    for mode, run in state["runs"].items():
        for layer, detail in run["reduction"]["tail_layers"].items():
            print(
                mode,
                "layer",
                layer,
                "elapsed_ms",
                detail["elapsed_ms"],
                "top modules",
                [
                    (e["module_id"], e["elapsed_ms"])
                    for e in detail["modules_by_elapsed"][:3]
                ],
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
