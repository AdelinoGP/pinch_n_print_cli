"""One authorized ordinary-only preparation attempt; never runs a benchmark.

Historical corpus and existing dist snapshot are preserved. A frozen manifest
is published only after every cell passes. Re-entry fails rather than retries.
Raw licensed inputs and large outputs stay under the ignored target tree.
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

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
HISTORICAL = ROOT / "tmp/rtree_query_corpus"
STATE = {
    "status": "preparing",
    "mode": "ordinary",
    "threads": 12,
    "purpose": "ordinary equivalence references, not geometry or adoption acceptance",
    "commands": [],
    "cells": [],
}


def utc():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


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


def corpus_files():
    return {
        str(path.relative_to(HISTORICAL)): digest(path)
        for path in sorted(HISTORICAL.rglob("*"))
        if path.is_file()
    }


def artifact_files(snapshot):
    return {
        str(path.relative_to(snapshot)): digest(path)
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
    with path.open(encoding="utf-8-sig") as stream:
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


def negative_controls():
    template = (
        "M83\n;LAYER_CHANGE\nG0 X0 Y0\n;TYPE:Outer wall\n"
        "G1 X10 Y0 E1\nG1 X10 Y10 E1\nG1 X0 Y10 E1\nG1 X0 Y0 E1\n"
        ";TYPE:Sparse infill\nG0 X{start} Y5\nG1 X{end} Y5 E1\n"
        "; CONFIG_BLOCK_START\n; test = control\n; CONFIG_BLOCK_END\n"
    )
    with tempfile.TemporaryDirectory(dir=ROOT / "target") as temporary:
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
    STATE["overshoot_controls"] = "inside accepted; outside start/end rejected"


def main():
    if (EVIDENCE / "preparation-state.json").exists():
        raise RuntimeError("preparation already attempted; no automatic retry")
    gates = json.loads((EVIDENCE / "preflight-results.json").read_text())
    if len(gates) != 5 or any(
        gate["exit_code"] or gate["missing_successful_tests"] for gate in gates
    ):
        raise RuntimeError("preparation prerequisites incomplete")
    negative_controls()
    STATE["started_utc"] = utc()
    scratch = (
        ROOT
        / "target/perimeter-reference-preparation"
        / datetime.now(timezone.utc).strftime("t40-%Y%m%dT%H%M%SZ")
    )
    scratch.mkdir(parents=True, exist_ok=False)
    STATE["scratch_root"] = str(scratch)
    before = source_inputs()
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
    if compiler != (EVIDENCE / "toolchain.log").read_text():
        raise RuntimeError("compiler changed after preflight")
    STATE["compiler_identity"] = compiler
    STATE["policy_source_sha256"] = digest(ROOT / "xtask/src/rustc_driver.rs")
    STATE["policy_doc_sha256"] = digest(ROOT / "docs/23_controlled_perimeter_builds.md")
    STATE["preparation_script_sha256"] = digest(Path(__file__).resolve())
    STATE["validator_script_sha256"] = digest(EVIDENCE / "validate-reference.ps1")
    STATE["shared_validator_sha256"] = digest(
        ROOT / "resources/perimeter-acceptance/validate_measurement.ps1"
    )
    STATE["cargo_configuration"] = {
        str(path): {"sha256": digest(path), "content": path.read_text()}
        for path in (ROOT / ".cargo/config.toml", Path.home() / ".cargo/config.toml")
        if path.is_file()
    }
    dist = ROOT / "target/dist/developer"
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
                if digest(input_path) != historical[f"{workload}/{input_path.name}"]:
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
                    str(EVIDENCE / "validate-reference.ps1"),
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
            if config["enable_support"] and not any(
                name.lower().startswith("support") for name in analysis["type_counts"]
            ):
                raise RuntimeError(
                    "support-enabled reference has no support TYPE evidence"
                )
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
        str(path.relative_to(corpus)): digest(path)
        for path in sorted(corpus.rglob("*"))
        if path.is_file()
    }
    STATE["status"] = "frozen"
    STATE["frozen_utc"] = utc()
    STATE["historical_corpus_unchanged"] = True
    STATE["build_inputs_unchanged"] = True
    save()
    write_json(EVIDENCE / "frozen-manifest.json", STATE)
    (EVIDENCE / "frozen-manifest.sha256").write_text(
        digest(EVIDENCE / "frozen-manifest.json") + "\n"
    )
    print(
        f"FROZEN: {corpus}; no campaign timing or accelerated validation ran.",
        flush=True,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        # Do not overwrite an earlier attempt when re-entry itself was rejected.
        if STATE.get("scratch_root"):
            STATE["status"] = "blocked"
            STATE["failure"] = str(error)
            save()
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)
