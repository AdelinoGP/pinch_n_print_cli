"""One-shot bounded capture driver; heavy payloads stay outside tracked evidence."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t53-union-simplify"
SOURCES = [
    "modules/core-modules/tree-support-planner/src/lib.rs",
    "crates/slicer-wasm-host/src/host.rs",
]
CONFIG = (
    "docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json"
)
MODES = [("ordinary", []), ("accelerated", ["--accelerated"])]


def sha(p):
    with Path(p).open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def write(p, data):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def inv(root):
    return {
        p.relative_to(root).as_posix(): sha(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def dist(mode):
    return REPO / (
        "target/dist/developer"
        if mode == "ordinary"
        else "target/dist-accelerated/developer"
    )


def run(command, name):
    log = RAW / (name + ".log")
    env = os.environ.copy()
    env.pop("SLICER_MODULE_PATH", None)
    env["RAYON_NUM_THREADS"] = "12"
    with log.open("wb") as f:
        r = subprocess.run(
            [str(c) for c in command],
            cwd=REPO,
            env=env,
            stdout=f,
            stderr=subprocess.STDOUT,
        )
    print(f"{name}: exit {r.returncode}; {log}", flush=True)
    return r.returncode


def checked(command, name):
    code = run(command, name)
    if code:
        raise RuntimeError(f"{name}: exit {code}; STOP (no retry)")
    return code


def prepare():
    if RAW.exists():
        raise RuntimeError("one-shot guard: raw directory already exists")
    for p in ["tmp/3dbenchy.stl", "tmp/base.stl", CONFIG, *SOURCES]:
        if not (REPO / p).is_file():
            raise RuntimeError(f"missing input: {p}")
    RAW.mkdir(parents=True)
    identity = {
        "head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        "compiler": subprocess.check_output(["rustc", "-vV"], text=True),
        "inputs": {
            p: sha(REPO / p) for p in ["tmp/3dbenchy.stl", "tmp/base.stl", CONFIG]
        },
        "sources": {p: sha(REPO / p) for p in SOURCES},
        "freshness": {},
        "snapshots": {},
    }
    (RAW / "initial-status.txt").write_bytes(
        subprocess.check_output(["git", "status", "--short"])
    )
    # Save exact files before any build. Never overwrite unrelated working-tree work.
    for p in SOURCES:
        dest = RAW / "before" / p
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / p, dest)
    artifacts = set()
    for mode, _ in MODES:
        if not dist(mode).is_dir():
            raise RuntimeError(f"missing active Developer distribution: {mode}")
        for p in dist(mode).rglob("*"):
            if p.is_file():
                artifacts.add(p)
    for pattern in [
        "modules/core-modules/**/*.wasm",
        "crates/slicer-wasm-host/test-guests/*.wasm",
        "target/guests*/artifacts/**/*.wasm",
        "target/guests*/wasm32-unknown-unknown/release/*.wasm",
        "target/guest-fingerprints*/*",
        "target/release/pnp_cli.exe",
        "target/dist-host-accelerated/release/pnp_cli.exe",
    ]:
        artifacts.update(p for p in REPO.glob(pattern) if p.is_file())
    inventory = {}
    for p in sorted(artifacts):
        rel = p.relative_to(REPO).as_posix()
        dest = RAW / "before" / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dest)
        inventory[rel] = sha(p)
    write(HERE / "before-inventory.json", inventory)
    write(HERE / "identity.json", identity)
    for mode, flags in MODES:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        code = run(command, mode + "-initial-freshness")
        identity["freshness"][mode] = {"command": command, "exit_code": code}
        write(HERE / "identity.json", identity)
        if code:
            raise RuntimeError(f"initial freshness failure: {mode}: {code}; STOP")
    # Build current-source baselines, rather than trusting an old staged host.
    for mode, flags in MODES:
        checked(
            ["cargo", "xtask", "dist", "--edition", "developer", *flags],
            mode + "-baseline-build",
        )
        snapshot = RAW / f"{mode}-baseline"
        shutil.copytree(dist(mode), snapshot)
        identity["snapshots"][mode] = inv(snapshot)
        checked(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            mode + "-baseline-freshness",
        )
        checked(
            [
                snapshot / "pnp_cli.exe",
                "module",
                "diagnose",
                "--module-dir",
                snapshot / "modules",
                "--no-default-module-paths",
                "--no-integrated-modules",
            ],
            mode + "-baseline-diagnose",
        )
        payload = json.loads((RAW / (mode + "-baseline-diagnose.log")).read_text())
        assert (
            payload["pass"]
            and payload["modules"]
            and all(m["provenance"] == "external" for m in payload["modules"])
        )
        identity.setdefault("diagnose", {})[mode] = payload
        write(HERE / "identity.json", identity)


def capture(arm):
    for mode, _ in MODES:
        snapshot = RAW / f"{mode}-{arm}"
        for job, model in [("benchy", "tmp/3dbenchy.stl"), ("base", "tmp/base.stl")]:
            name = f"{mode}-{job}-{arm}"
            output = RAW / (name + ".gcode")
            if output.exists():
                raise RuntimeError(f"one-shot guard: {name}")
            checked(
                [
                    snapshot / "pnp_cli.exe",
                    "slice",
                    "--model",
                    REPO / model,
                    "--config",
                    REPO / CONFIG,
                    "--module-dir",
                    snapshot / "modules",
                    "--no-default-module-paths",
                    "--no-integrated-modules",
                    "--output",
                    output,
                    "--instrument-stderr",
                ],
                name,
            )
            # Qualification is checked immediately; no next cell after output drift.
            import verify

            row = verify.output_row(output, RAW / (name + ".log"))
            if arm == "probe":
                baseline = RAW / (f"{mode}-{job}-baseline.gcode")
                assert sha(output) == sha(baseline), f"OUTPUT DRIFT: {name}; STOP"
            write(HERE / (name + "-output.json"), row)


def build_probe():
    (HERE / "probe.patch").write_bytes(
        subprocess.check_output(["git", "--no-pager", "diff", "--", *SOURCES])
    )
    for mode, flags in MODES:
        checked(
            ["cargo", "xtask", "dist", "--edition", "developer", *flags],
            mode + "-probe-build",
        )
        checked(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            mode + "-probe-freshness",
        )
        snapshot = RAW / f"{mode}-probe"
        if snapshot.exists():
            raise RuntimeError("one-shot guard")
        shutil.copytree(dist(mode), snapshot)
        checked(
            [
                snapshot / "pnp_cli.exe",
                "module",
                "diagnose",
                "--module-dir",
                snapshot / "modules",
                "--no-default-module-paths",
                "--no-integrated-modules",
            ],
            mode + "-probe-diagnose",
        )
        payload = json.loads((RAW / (mode + "-probe-diagnose.log")).read_text())
        assert payload["pass"] and all(
            m["provenance"] == "external" for m in payload["modules"]
        )
        write(HERE / (mode + "-probe-inventory.json"), inv(snapshot))


def restore():
    for p in SOURCES:
        shutil.copy2(RAW / "before" / p, REPO / p)
    result = {
        "sources": {p: sha(REPO / p) for p in SOURCES},
        "freshness": {},
        "artifacts_match_before": False,
    }
    # Rebuild restored inputs to put Cargo caches/fingerprints back in production state.
    for mode, flags in MODES:
        checked(["cargo", "xtask", "build-guests", *flags], mode + "-restored-build")
    inventory = json.loads((HERE / "before-inventory.json").read_text())
    for p in inventory:
        shutil.copy2(RAW / "before" / p, REPO / p)
    assert all(sha(REPO / p) == v for p, v in inventory.items())
    result["artifacts_match_before"] = True
    for mode, flags in MODES:
        cmd = ["cargo", "xtask", "build-guests", *flags, "--check"]
        result["freshness"][mode] = {
            "command": cmd,
            "exit_code": run(cmd, mode + "-restored-freshness"),
        }
    result["production_diff"] = subprocess.check_output(
        ["git", "diff", "--", *SOURCES], text=True
    )
    write(HERE / "restoration.json", result)
    assert not result["production_diff"] and all(
        v["exit_code"] == 0 for v in result["freshness"].values()
    )


if __name__ == "__main__":
    {
        "prepare": prepare,
        "baseline": lambda: capture("baseline"),
        "build-probe": build_probe,
        "probe": lambda: capture("probe"),
        "restore": restore,
    }[sys.argv[1]]()
