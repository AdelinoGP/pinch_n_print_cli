"""Isolated exact-version shared-engine experiment; immutable arm snapshots."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t57-topx-cache"
SOURCE = RAW / "source"
LIB = SOURCE / ".local-libs/clipper2-rust"
MODES = [("ordinary", []), ("accelerated", ["--accelerated"])]


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inv(root):
    return {
        p.relative_to(root).as_posix(): sha(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def write(path, value):
    # Atomic checkpoint: a kill mid-write must never corrupt the previous file.
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def run(command, name, cwd=REPO, env=None):
    log = HERE / "logs" / (name + ".log")
    log.parent.mkdir(parents=True, exist_ok=True)
    effective = {**os.environ, "RAYON_NUM_THREADS": "12", **(env or {})}
    effective.pop("SLICER_MODULE_PATH", None)
    with log.open("wb") as out:
        result = subprocess.run(
            [str(c) for c in command],
            cwd=cwd,
            env=effective,
            stdout=out,
            stderr=subprocess.STDOUT,
        )
    print(f"{name}: exit {result.returncode}; {log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"STOP: {name}, exit {result.returncode}; inspect saved log")
    return result.returncode


def prepare(resume=False):
    if resume:
        assert sorted(p.name for p in RAW.iterdir()) == ["initial-status.txt"], (
            "unexpected incomplete preparation state"
        )
        blocker = json.loads((HERE / "preflight-blocker.json").read_text())
        assert (
            blocker["stage"] == "ordinary-current-baseline-provenance"
            and not blocker["candidate_started"]
        )
    else:
        assert not RAW.exists(), "one-shot preparation guard"
        RAW.mkdir(parents=True)
    prior = json.loads((HERE.parent / "t55-body-union/identity.json").read_text())
    audited = json.loads(
        (HERE.parent / "t55-body-union/archive-audit.json").read_text()
    )
    restored_inventory = json.loads(
        (HERE.parent / "t55-body-union/before-inventory.json").read_text()
    )
    assert all(
        sha(REPO / p) == digest for p, digest in audited["build_inputs"].items()
    ), "current build-input drift"
    assert all(sha(REPO / p) == digest for p, digest in prior["inputs"].items()), (
        "matched input drift"
    )
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    assert compiler == prior["compiler"], "compiler drift"
    (RAW / "initial-status.txt").write_bytes(
        subprocess.check_output(["git", "status", "--short"], cwd=REPO)
    )
    identity = {
        "compiler": compiler,
        "build_inputs": audited["build_inputs"],
        "inputs": prior["inputs"],
        "freshness": {},
        "baseline_snapshots": {},
    }
    for mode, flags in MODES:
        identity["freshness"][mode] = run(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            mode + ("-preflight-corrected" if resume else "-preflight"),
        )
        dist = REPO / (
            "target/dist/developer"
            if mode == "ordinary"
            else "target/dist-accelerated/developer"
        )
        prefix = dist.relative_to(REPO).as_posix() + "/"
        expected = {
            p[len(prefix) :]: digest
            for p, digest in restored_inventory.items()
            if p.startswith(prefix)
        }
        assert expected and inv(dist) == expected, (
            f"current {mode} baseline provenance drift"
        )
        snapshot = RAW / "snapshots" / (mode + "-baseline")
        shutil.copytree(dist, snapshot)
        identity["baseline_snapshots"][mode] = inv(snapshot)
    # Save exact guest artifacts, host binaries and fingerprints before shared-cache builds.
    patterns = [
        "modules/core-modules/**/*.wasm",
        "crates/slicer-wasm-host/test-guests/*.wasm",
        "target/guests*/artifacts/**/*.wasm",
        "target/guests*/wasm32-unknown-unknown/release/*.wasm",
        "target/guest-fingerprints*/*",
        "target/dist/developer/**/*",
        "target/dist-accelerated/developer/**/*",
        "target/release/pnp_cli.exe",
        "target/dist-host-accelerated/release/pnp_cli.exe",
    ]
    saved = {}
    for p in sorted(
        {p for pattern in patterns for p in REPO.glob(pattern) if p.is_file()}
    ):
        rel = p.relative_to(REPO)
        dst = RAW / "before" / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dst)
        saved[rel.as_posix()] = sha(p)
    identity["before_artifacts"] = saved
    # Copy current tracked build resources, not a commit checkout: preserve dirty inputs.
    paths = (
        subprocess.check_output(["git", "ls-files", "-z"], cwd=REPO)
        .decode()
        .split("\0")
    )
    prefixes = ("crates/", "modules/", "xtask/", "resources/", "dist/", ".cargo/")
    copied = {}
    for rel in paths:
        if not rel or not (("/" not in rel) or rel.startswith(prefixes)):
            continue
        p = REPO / rel
        if not p.is_file():
            continue
        dst = SOURCE / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(p, dst)
        copied[rel] = sha(p)
    identity["source_snapshot"] = copied
    registry = list((Path.home() / ".cargo/registry/src").glob("*/clipper2-rust-1.1.0"))
    assert len(registry) == 1
    shutil.copytree(registry[0], LIB)
    identity["registry_path"] = str(registry[0])
    identity["registry_inventory"] = inv(registry[0])
    # Reuse dependency compilation caches, never use active distributions as timing arms.
    run(
        [
            "pwsh",
            "-NoProfile",
            "-Command",
            "New-Item -ItemType Junction -Path '"
            + str(SOURCE / "target")
            + "' -Target '"
            + str(REPO / "target")
            + "' | Out-Null",
        ],
        "target-junction",
    )
    write(HERE / "identity.json", identity)
    if resume:
        blocker["resolution"] = (
            "driver compared diagnostic-built host with restored production; corrected authority is t55 before-inventory plus restoration, all actual restored files match"
        )
        blocker["resolved_before_candidate_work"] = True
        write(HERE / "preflight-blocker.json", blocker)
    print(
        "PASS: current-source baselines, artifacts and registry saved; candidate source snapshot isolated",
        flush=True,
    )


def restore():
    identity = json.loads((HERE / "identity.json").read_text())
    assert all(
        sha(REPO / p) == digest for p, digest in identity["build_inputs"].items()
    ), "production build inputs changed"
    assert inv(Path(identity["registry_path"])) == identity["registry_inventory"], (
        "registry changed"
    )
    for rel in identity["before_artifacts"]:
        shutil.copy2(RAW / "before" / rel, REPO / rel)
    assert all(
        sha(REPO / p) == digest for p, digest in identity["before_artifacts"].items()
    )
    restored = {
        "build_inputs_unchanged": True,
        "registry_unchanged": True,
        "archived_artifacts_byte_restored": True,
        "freshness": {},
    }
    for mode, flags in MODES:
        restored["freshness"][mode] = run(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            mode + "-restored-freshness",
        )
        dist = REPO / (
            "target/dist/developer"
            if mode == "ordinary"
            else "target/dist-accelerated/developer"
        )
        assert inv(dist) == identity["baseline_snapshots"][mode], (
            "distribution file-set drift"
        )
    write(HERE / "restoration.json", restored)
    print(
        "PASS: production sources unchanged, artifacts byte-restored, both freshness gates clean",
        flush=True,
    )


if __name__ == "__main__":
    {
        "prepare": prepare,
        "resume-preparation": lambda: prepare(True),
        "restore": restore,
    }[sys.argv[1]]()
