"""One authorized t41 ordinary-only preparation attempt; never benchmarks.

Human-authorized fresh re-preparation after the t40 frozen payload roots
disappeared (issue 41). Differences from t40's prepare.py, which stays
untouched:

* The payload root is durable: `.local-artifacts/perimeter-reference-
  preparation/t41-<UTC>/` (gitignored, outside `target/`). The regression in
  `test-retention.py` proves a target-nested root does not survive a build
  `target/` cleanup while this one does.
* The manifest records the exact t41 tool-set hashes (all scripts), so the
  read-only verifier can reject a swapped/edited tool without re-freezing
  anything.

Historical corpus and any existing dist snapshot are preserved. A frozen
manifest is published only after every cell passes. Re-entry fails rather
than retries. Raw licensed inputs and large outputs stay in the durable,
untracked payload root; they are not scratch and must not be cleaned.
"""

from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile

BASE_EVIDENCE = Path(__file__).resolve().parent
EVIDENCE = BASE_EVIDENCE
ROOT = BASE_EVIDENCE.parents[4]
HISTORICAL = ROOT / "tmp/rtree_query_corpus"
DURABLE = ROOT / ".local-artifacts/perimeter-reference-preparation"
TOOL_FILES = (
    "prepare.py",
    "preflight.py",
    "verify-freeze.py",
    "validate-reference.ps1",
    "test-preparation.py",
    "test-freeze.py",
    "test-retention.py",
)
STATE = {
    "status": "preparing",
    "mode": "ordinary",
    "threads": 12,
    "purpose": (
        "ordinary equivalence references, not geometry or adoption acceptance; "
        "re-preparation after missing target-nested frozen roots (issue 41)"
    ),
    "commands": [],
    "cells": [],
}


def utc():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def save():
    write_json(EVIDENCE / "preparation-state.json", STATE)


def run(name, command):
    print(f"CHECK {name}", flush=True)
    with (EVIDENCE / f"{name}.log").open("wb") as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
    STATE["commands"].append(
        {"name": name, "argv": command, "exit_code": result.returncode}
    )
    save()
    if result.returncode:
        raise RuntimeError(f"{name} failed: exit {result.returncode}; see {name}.log")
    return (EVIDENCE / f"{name}.log").read_text(encoding="utf-8-sig")


def source_inputs():
    """Tracked/untracked build-relevant inputs; docs/ and .agents/ excluded.

    `test-preparation.py` and `verify-freeze.py` run this through `runpy`, so
    it must stay side-effect free.
    """
    names = (
        subprocess.check_output(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=ROOT,
        )
        .decode()
        .split("\0")
    )
    files = {
        name: digest(ROOT / name)
        for name in sorted(set(names))
        if name
        and (ROOT / name).is_file()
        and not name.startswith(("docs/", ".agents/"))
        and (
            Path(name).suffix in (".rs", ".toml", ".wit")
            or Path(name).name == "Cargo.lock"
        )
    }
    for config in (Path.home() / ".cargo/config", Path.home() / ".cargo/config.toml"):
        if config.is_file():
            files[str(config)] = digest(config)
    return files


def relative_key(path, root):
    """Portable manifest key, also used by input lookup on Windows."""
    return path.relative_to(root).as_posix()


def corpus_files():
    return {
        relative_key(path, HISTORICAL): digest(path)
        for path in sorted(HISTORICAL.rglob("*"))
        if path.is_file()
    }


def artifact_files(snapshot):
    return {
        relative_key(path, snapshot): digest(path)
        for path in sorted(snapshot.rglob("*"))
        if path.is_file()
    }


def check_output(path):
    """Check both sparse-segment endpoints against deposited-wall bboxes.

    No polygon-containment claim. Travels never enlarge the wall envelope;
    layers lacking wall evidence fail instead of silently skipping sparse fill.
    """
    layers = {}
    role = None
    x = y = None
    layer = -1
    config_blocks = 0
    in_config = False
    settings = {}
    types = Counter()
    with Path(path).open(encoding="utf-8-sig") as stream:
        for line in stream:
            if line.startswith((";LAYER_CHANGE", "; CHANGE_LAYER")):
                layer += 1
                role = None
                layers[layer] = {"walls": [], "sparse": [], "segments": 0}
            if line.strip() == "; CONFIG_BLOCK_START":
                config_blocks += 1
                in_config = True
            elif line.strip() == "; CONFIG_BLOCK_END":
                in_config = False
            elif in_config:
                match = re.fullmatch(r";\s*([^=]+?)\s*=\s*(.*?)\s*", line.rstrip())
                if match:
                    key, value = match.groups()
                    if key in settings:
                        raise RuntimeError(f"duplicate appendix key: {key}")
                    settings[key] = value
            if line.startswith(";TYPE:"):
                role = line[len(";TYPE:") :].strip()
                types[role] += 1
            fields = line.split(";", 1)[0].split()
            if not fields:
                continue
            if fields[0] in ("G91", "M82"):
                raise RuntimeError(
                    f"unsupported coordinate/extrusion mode: {fields[0]}"
                )
            if fields[0] == "G92":
                for field in fields[1:]:
                    if field.startswith("X"):
                        x = float(field[1:])
                    elif field.startswith("Y"):
                        y = float(field[1:])
            if fields[0] not in ("G0", "G1"):
                continue
            values = {
                field[0]: float(field[1:]) for field in fields[1:] if field[0] in "XYE"
            }
            nx, ny = values.get("X", x), values.get("Y", y)
            if not all(math.isfinite(v) for v in values.values()):
                raise RuntimeError("nonfinite motion value")
            if (
                layer >= 0
                and fields[0] == "G1"
                and values.get("E", 0) > 0
                and ("X" in values or "Y" in values)
            ):
                if None in (x, y, nx, ny):
                    raise RuntimeError("extruding segment lacks coordinates")
                endpoints = [(x, y), (nx, ny)]
                if role in ("Outer wall", "Inner wall"):
                    layers[layer]["walls"].extend(endpoints)
                elif role == "Sparse infill" and (x, y) != (nx, ny):
                    layers[layer]["sparse"].extend(endpoints)
                    layers[layer]["segments"] += 1
            x, y = nx, ny
    if in_config or config_blocks != 1 or not layers:
        raise RuntimeError("missing/incomplete config block or layer evidence")
    count = sum(data["segments"] for data in layers.values())
    if count == 0:
        raise RuntimeError("no sparse-segment evidence for the positive-density job")
    worst = 0.0
    for index, data in layers.items():
        if not data["sparse"]:
            continue
        if not data["walls"]:
            raise RuntimeError(f"sparse layer {index} has no deposited-wall envelope")
        xs, ys = zip(*data["walls"])
        min_x, max_x, min_y, max_y = min(xs), max(xs), min(ys), max(ys)
        for px, py in data["sparse"]:
            dx = max(min_x - px, px - max_x, 0.0)
            dy = max(min_y - py, py - max_y, 0.0)
            worst = max(worst, math.hypot(dx, dy))
    result = {
        "layers": len(layers),
        "sparse_segments": count,
        "worst_out_of_deposited_wall_bbox_mm": worst,
        "type_counts": dict(types),
        "appendix": settings,
        "limitation": "wall bounding box only; not polygon containment",
    }
    if worst > 0:
        raise RuntimeError(f"sparse overshoot detected: {worst} mm")
    return result


def check_settings(config, appendix):
    for key, expected in {
        **config,
        "infill_density": 0.2,
        "sparse_fill_holder": "rectilinear-infill",
        "wall_loops": 3,
    }.items():
        if key not in appendix:
            raise RuntimeError(f"missing supported-setting disclosure: {key}")
        actual = appendix[key]
        if isinstance(expected, bool):
            matches = actual.lower() == str(expected).lower()
        elif isinstance(expected, (float, int)):
            # Permit the exact configured value or its exact f32 promotion,
            # not an arbitrary geometry/config-equivalence tolerance.
            f32 = struct.unpack("f", struct.pack("f", expected))[0]
            matches = float(actual) in (expected, f32)
        else:
            matches = actual.lower() == str(expected).lower()
        if not matches:
            raise RuntimeError(
                f"setting conflict: {key}, requested {expected}, disclosed {actual}"
            )


def support_type_evidence(config, analysis):
    """Support-enabled cells must show a support TYPE role in the G-code."""
    if not config["enable_support"]:
        return False
    if not any(name.lower().startswith("support") for name in analysis["type_counts"]):
        raise RuntimeError("support-enabled reference has no support TYPE evidence")
    return True


def negative_controls():
    """Positive control accepted; outside-start and outside-end rejected."""
    template = (
        "M83\n;LAYER_CHANGE\nG0 X0 Y0\n;TYPE:Outer wall\n"
        "G1 X10 Y0 E1\nG1 X10 Y10 E1\nG1 X0 Y10 E1\nG1 X0 Y0 E1\n"
        ";TYPE:Sparse infill\nG0 X{start} Y5\nG1 X{end} Y5 E1\n"
        "; CONFIG_BLOCK_START\n; test = control\n; CONFIG_BLOCK_END\n"
    )
    missing_envelope = (
        "M83\n;LAYER_CHANGE\nG0 X0 Y0\n"
        ";TYPE:Sparse infill\nG0 X1 Y5\nG1 X9 Y5 E1\n"
        "; CONFIG_BLOCK_START\n; test = control\n; CONFIG_BLOCK_END\n"
    )
    with tempfile.TemporaryDirectory() as temporary:
        path = Path(temporary) / "control.gcode"
        path.write_text(template.format(start=1, end=9))
        check_output(path)
        for start, end in ((1, 11), (11, 1)):
            path.write_text(template.format(start=start, end=end))
            try:
                check_output(path)
            except RuntimeError as error:
                if "overshoot detected" not in str(error):
                    raise
            else:
                raise RuntimeError("overshoot negative control falsely passed")
        path.write_text(missing_envelope)
        try:
            check_output(path)
        except RuntimeError as error:
            if "no deposited-wall envelope" not in str(error):
                raise
        else:
            raise RuntimeError("missing-wall-envelope control falsely passed")
    STATE["overshoot_controls"] = (
        "inside accepted; outside start/end rejected; sparse-without-wall rejected"
    )


def load_preflight_gates():
    """Fresh t41 prerequisites only; never consumes t40 evidence."""
    results = json.loads((BASE_EVIDENCE / "preflight-results.json").read_text())
    if results.get("status") != "passed":
        raise RuntimeError("preflight status is not 'passed'")
    freshness = results.get("checks", {}).get("freshness", {})
    if freshness.get("final_exit") != 0:
        raise RuntimeError("preflight freshness did not reach exit 0")
    if not results.get("checks", {}).get("target_inspection", {}).get("ok"):
        raise RuntimeError("preflight target inspection did not pass")
    gates = {gate["name"]: gate for gate in results.get("gates", [])}
    for name in ("support-repair", "empty-infill-producers", "empty-infill-runtime"):
        gate = gates.get(name)
        if gate is None:
            raise RuntimeError(f"preparation prerequisites missing gate: {name}")
        counts = gate["passed_counts"]
        if (
            gate["exit_code"]
            or gate["missing_successful_tests"]
            or counts["failed"]
            or counts["passed"] != gate.get("expected_passed_exact")
        ):
            raise RuntimeError(f"preparation prerequisite not clean: {name}: {counts}")
    return results


def main():
    if (EVIDENCE / "preparation-state.json").exists():
        raise RuntimeError("preparation already attempted; no automatic retry")
    load_preflight_gates()
    negative_controls()
    STATE["started_utc"] = utc()
    scratch = DURABLE / datetime.now(timezone.utc).strftime("t41-%Y%m%dT%H%M%SZ")
    scratch.mkdir(parents=True, exist_ok=False)
    if not scratch.is_relative_to(ROOT / ".local-artifacts"):
        raise RuntimeError("payload root escaped the durable namespace")
    STATE["payload_root"] = str(scratch)
    STATE["durable_namespace"] = str(DURABLE)
    save()
    before = source_inputs()
    if before != json.loads((BASE_EVIDENCE / "source-inputs.json").read_text()):
        raise RuntimeError(
            "build inputs changed since prerequisite tests; re-establish gates explicitly"
        )
    historical = corpus_files()
    write_json(EVIDENCE / "source-inputs.json", before)
    write_json(EVIDENCE / "historical-corpus-hashes.json", historical)
    STATE["source_revision"] = (
        subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip()
    )
    (EVIDENCE / "source-worktree.diff").write_bytes(
        subprocess.check_output(
            ["git", "--no-pager", "diff", "--binary", "HEAD"], cwd=ROOT
        )
    )
    (EVIDENCE / "source-worktree-status.txt").write_bytes(
        subprocess.check_output(["git", "status", "--short"], cwd=ROOT)
    )
    for key in (
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "PNP_ACCELERATED",
        "PNP_ACCELERATED_MODE",
        "SLICER_MODULE_PATH",
    ):
        if os.environ.get(key):
            raise RuntimeError(f"unexpected ordinary environment: {key}")
    compiler = run("snapshot-toolchain", ["rustc", "-vV"])
    if compiler != (BASE_EVIDENCE / "toolchain.log").read_text():
        raise RuntimeError("compiler changed after preflight")
    STATE["compiler_identity"] = compiler
    STATE["policy_source_sha256"] = digest(ROOT / "xtask/src/rustc_driver.rs")
    STATE["policy_doc_sha256"] = digest(ROOT / "docs/23_controlled_perimeter_builds.md")
    STATE["tools"] = {name: digest(BASE_EVIDENCE / name) for name in TOOL_FILES}
    STATE["shared_validator_sha256"] = digest(
        ROOT / "resources/perimeter-acceptance/validate_measurement.ps1"
    )
    STATE["cargo_configuration"] = {
        str(path): {"sha256": digest(path), "content": path.read_text()}
        for path in (ROOT / ".cargo/config.toml", Path.home() / ".cargo/config.toml")
        if path.is_file()
    }
    dist = ROOT / "target/dist/developer"
    STATE["previous_dist_present"] = dist.exists()
    if dist.exists():
        shutil.copytree(dist, scratch / "previous-dist-preserved")
    run("ordinary-dist-build", ["cargo", "xtask", "dist", "--edition", "developer"])
    run("snapshot-ordinary-freshness", ["cargo", "xtask", "build-guests", "--check"])
    if source_inputs() != before:
        raise RuntimeError("build inputs changed during snapshot build")
    snapshot = scratch / "ordinary"
    shutil.copytree(dist, snapshot)
    snapshot_hashes = artifact_files(snapshot)
    if snapshot_hashes != artifact_files(dist):
        raise RuntimeError("snapshot copy identity mismatch")
    STATE["snapshot_root"] = str(snapshot)
    STATE["snapshot_sha256"] = snapshot_hashes
    corpus = scratch / "corpus"
    corpus.mkdir()
    STATE["corpus_root"] = str(corpus)
    STATE["preflight_source_inputs_sha256"] = digest(
        BASE_EVIDENCE / "source-inputs.json"
    )
    save()
    for workload in ("supports-off-benchy", "tree-support-benchy", "tree-support-base"):
        original = HISTORICAL / workload
        prepared = corpus / workload
        prepared.mkdir()
        shutil.copy2(original / "model.stl", prepared / "model.stl")
        for generator in ("classic", "arachne"):
            config_path = prepared / f"{generator}.json"
            shutil.copy2(original / config_path.name, config_path)
            config = json.loads(config_path.read_text(encoding="utf-8-sig"))
            for input_path in (prepared / "model.stl", config_path):
                if digest(input_path) != historical[relative_key(input_path, corpus)]:
                    raise RuntimeError("model/config copy mismatch")
            output = prepared / f"reference-{generator}.gcode"
            stderr = prepared / f"reference-{generator}.jsonl"
            stdout = prepared / f"reference-{generator}.stdout.txt"
            command = [
                str(snapshot / "pnp_cli.exe"),
                "slice",
                "--model",
                str(prepared / "model.stl"),
                "--config",
                str(config_path),
                "--output",
                str(output),
                "--module-dir",
                str(snapshot / "modules"),
                "--no-default-module-paths",
                "--no-integrated-modules",
            ]
            cell = {
                "workload": workload,
                "generator": generator,
                "command": command,
                "started_utc": utc(),
                "config": config,
                "status": "generating",
            }
            STATE["cells"].append(cell)
            save()
            print(f"PREPARE ordinary {workload}/{generator} (no benchmark)", flush=True)
            with stdout.open("wb") as out, stderr.open("wb") as err:
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    env={**os.environ, "RAYON_NUM_THREADS": "12"},
                    stdout=out,
                    stderr=err,
                )
            cell["exit_code"] = result.returncode
            cell["generated_utc"] = utc()
            save()
            if result.returncode:
                raise RuntimeError(
                    f"ordinary slice failed: {workload}/{generator}, exit {result.returncode}"
                )
            validation = run(
                f"{workload}-{generator}-status",
                [
                    "pwsh",
                    "-NoProfile",
                    "-File",
                    str(BASE_EVIDENCE / "validate-reference.ps1"),
                    "-ConfigPath",
                    str(config_path),
                    "-OutputPath",
                    str(output),
                    "-StderrPath",
                    str(stderr),
                    "-ExpectedGenerator",
                    generator,
                    "-ExitCode",
                    str(result.returncode),
                ],
            )
            cell["completion_evidence"] = json.loads(validation)
            analysis = check_output(output)
            check_settings(config, analysis["appendix"])
            cell["support_type_evidence"] = support_type_evidence(config, analysis)
            cell["output_checks"] = analysis
            cell["files_sha256"] = {
                path.name: digest(path)
                for path in (
                    prepared / "model.stl",
                    config_path,
                    output,
                    stderr,
                    stdout,
                )
            }
            cell["status"] = "validated"
            save()
            print(
                f"PASS {workload}/{generator}: clean status, settings, sparse bbox",
                flush=True,
            )
    if (
        source_inputs() != before
        or artifact_files(snapshot) != snapshot_hashes
        or corpus_files() != historical
    ):
        raise RuntimeError(
            "source, snapshot or historical corpus identity changed during preparation"
        )
    STATE["corpus_sha256"] = {
        relative_key(path, corpus): digest(path)
        for path in sorted(corpus.rglob("*"))
        if path.is_file()
    }
    STATE["preflight_gates_sha256"] = digest(BASE_EVIDENCE / "preflight-results.json")
    STATE["status"] = "frozen"
    STATE["frozen_utc"] = utc()
    STATE["historical_corpus_unchanged"] = True
    STATE["build_inputs_unchanged"] = True
    STATE["source_inputs_sha256"] = digest(EVIDENCE / "source-inputs.json")
    save()
    write_json(EVIDENCE / "frozen-manifest.json", STATE)
    (EVIDENCE / "frozen-manifest.sha256").write_text(
        digest(EVIDENCE / "frozen-manifest.json") + "\n"
    )
    write_json(
        EVIDENCE / "tool-provenance.json",
        {
            "tools": STATE["tools"],
            "shared_validator_sha256": STATE["shared_validator_sha256"],
            "policy_source_sha256": STATE["policy_source_sha256"],
            "policy_doc_sha256": STATE["policy_doc_sha256"],
            "preflight_gates_sha256": STATE["preflight_gates_sha256"],
        },
    )
    print(
        f"FROZEN: {corpus}; payload {scratch}; no campaign timing or "
        "accelerated validation ran.",
        flush=True,
    )


if __name__ == "__main__":
    owned_attempt = False
    try:
        if (
            len(sys.argv) != 3
            or sys.argv[1] != "--attempt"
            or not re.fullmatch(r"[a-z0-9-]+", sys.argv[2])
        ):
            raise RuntimeError(
                "new attempt requires explicit --attempt <new-name>; never retries in place"
            )
        EVIDENCE = BASE_EVIDENCE / sys.argv[2]
        EVIDENCE.mkdir(exist_ok=False)
        owned_attempt = True
        main()
    except Exception as error:
        # Record the blocked state for THIS run's own attempt, preserving it for
        # the human; never touch an attempt this run did not create (re-entry
        # rejection) and never demote a completed frozen attempt.
        try:
            if owned_attempt and not (EVIDENCE / "frozen-manifest.json").exists():
                STATE["status"] = "blocked"
                STATE["failure"] = str(error)
                save()
        except OSError:
            pass
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)
