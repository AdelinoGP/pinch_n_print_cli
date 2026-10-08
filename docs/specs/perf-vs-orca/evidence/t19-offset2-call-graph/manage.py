"""Preserve HEAD snapshots, stage the one-guest candidate, run the standing A/B.

Ticket 19: narrow-island opening-emptiness short-circuit in classic-perimeters.
Raw captures live under the gitignored target/t19-ab/campaign tree; the in-repo
reduction is verify-t19.py.
"""

import csv
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target/t19-ab/campaign"
SOURCES = [
    "Cargo.toml",
    "modules/core-modules/classic-perimeters/src/lib.rs",
    "modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs",
    "crates/slicer-core/src/polygon_ops.rs",
    "crates/slicer-core/tests/opening_emptiness_tdd.rs",
]
GUEST = "modules/classic-perimeters/classic-perimeters.wasm"
MODEL_SHA = "6a07f34cc7769b1c852635212c91a1b354532f4222a9a8105c1035a1a7b284f7"
CONFIG = "docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-off.json"
REFERENCE_GCODE_SHA = "34886ccb56b214d57c28aa58bdcc8d371a41f440200f5b49276b96b53e18cc40"


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def inventory(root):
    return {
        p.relative_to(root).as_posix(): digest(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def dist(mode):
    return REPO / (
        "target/dist/developer"
        if mode == "ordinary"
        else "target/dist-accelerated/developer"
    )


def checked(command, log):
    with log.open("w", encoding="utf-8") as stream:
        result = subprocess.run(
            command, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT
        )
    if result.returncode:
        raise RuntimeError(f"exit {result.returncode}: {command}; see {log}")
    print(f"PASS: {' '.join(command)}", flush=True)
    return result


def tee(command, log):
    """Run a command with combined output tee'd to target/test-output.log."""
    target_log = REPO / "target/test-output.log"
    if target_log.exists():
        target_log.unlink()
    result = checked(command, target_log)
    shutil.copy2(target_log, log)
    return result


def guest_source(mode):
    return REPO / (
        "modules/core-modules/classic-perimeters/classic-perimeters.wasm"
        if mode == "ordinary"
        else "target/guests-accelerated/artifacts/modules/core-modules/classic-perimeters/classic-perimeters.wasm"
    )


def prepare():
    """Freeze the current (HEAD) distributions as both modes' baselines."""
    if RAW.exists():
        raise RuntimeError(f"one-shot guard: {RAW}")
    model = REPO / "tmp/3dbenchy.stl"
    config = REPO / CONFIG
    if digest(model) != MODEL_SHA:
        raise RuntimeError("Benchy identity differs from the frozen job")
    RAW.mkdir(parents=True)
    (RAW / "snapshots").mkdir()
    identity = {
        "baseline_head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=REPO, text=True
        ).strip(),
        "model_sha256": digest(model),
        "config": CONFIG,
        "config_sha256": digest(config),
        "reference_gcode_sha256": REFERENCE_GCODE_SHA,
        "compiler": subprocess.check_output(["rustc", "-vV"], text=True),
        "freshness": {},
        "host_binary_sha256": {},
        "snapshots": {},
        "baseline_artifacts": {},
    }
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        # Both distributions were staged from the current tree before this
        # snapshot; prove the mode's guest artifact is the one the freshness
        # gate validated, then freeze the whole tree by inventory.
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        checked(command, RAW / f"{mode}-preflight.log")
        identity["freshness"][mode] = {"command": command, "exit_code": 0}
        root = dist(mode)
        artifact = root / GUEST
        if digest(artifact) != digest(guest_source(mode)):
            raise RuntimeError(
                f"{mode} dist guest differs from the freshness-checked artifact"
            )
        identity["host_binary_sha256"][mode] = digest(
            root / ("pnp_cli.exe" if (root / "pnp_cli.exe").exists() else "pnp_cli")
        )
        identity["baseline_artifacts"][mode] = digest(artifact)
        dest = RAW / f"snapshots/{mode}-baseline"
        shutil.copytree(root, dest)
        identity["snapshots"][mode] = inventory(dest)
    (RAW / "snapshots/head.txt").write_text(identity["baseline_head"] + "\n")
    write(HERE / "identity.json", identity)
    write(RAW / "identity.json", identity)
    print("HEAD distributions snapshotted and frozen.", flush=True)


def stage(mode):
    baseline = RAW / f"snapshots/{mode}-baseline"
    candidate = RAW / f"snapshots/{mode}-candidate"
    if candidate.exists():
        raise RuntimeError(f"one-shot guard: {candidate}")
    shutil.copytree(baseline, candidate)
    shutil.copy2(guest_source(mode), candidate / GUEST)
    arms_path = RAW / "snapshots/arm-inventory.json"
    arms = json.loads(arms_path.read_text()) if arms_path.exists() else {}
    a, b = inventory(baseline), inventory(candidate)
    changes = [k for k in a if a[k] != b[k]]
    if a.keys() != b.keys() or changes != [GUEST]:
        raise RuntimeError(f"non-isolated candidate: {mode}: {changes}")
    arms[mode] = {"baseline": a, "candidate": b, "changed": changes}
    write(arms_path, arms)
    print(f"Staged isolated {mode} classic-perimeters guest.", flush=True)


def build_candidates():
    # Ordinary leg: rebuild the shared + in-tree guest from the candidate source.
    checked(["cargo", "xtask", "build-guests"], RAW / "ordinary-candidate-build.log")
    checked(
        ["cargo", "xtask", "build-guests", "--check"],
        RAW / "ordinary-candidate-freshness.log",
    )
    stage("ordinary")
    # Accelerated leg.
    checked(
        ["cargo", "xtask", "build-guests", "--accelerated"],
        RAW / "accelerated-candidate-build.log",
    )
    checked(
        ["cargo", "xtask", "build-guests", "--accelerated", "--check"],
        RAW / "accelerated-candidate-freshness.log",
    )
    stage("accelerated")


def tests():
    for name, command in [
        (
            "core-exactness-tests",
            [
                "cargo",
                "test",
                "-p",
                "slicer-core",
                "--test",
                "opening_emptiness_tdd",
                "--test",
                "offset2_ex_tdd",
                "--test",
                "offset2_ex_collapse_tdd",
            ],
        ),
        (
            "guest-suite",
            [
                "cargo",
                "test",
                "-p",
                "classic-perimeters",
                "--test",
                "classic_perimeters_tdd",
                "--test",
                "classic_perimeters_concave_tdd",
                "--test",
                "wall_closure_tdd",
                "--test",
                "boundary_paint_tdd",
                "--lib",
            ],
        ),
    ]:
        tee(command, HERE / f"{name}.log")


def gates():
    for name, command in [
        ("workspace-build", ["cargo", "build", "--workspace"]),
        (
            "clippy",
            [
                "cargo",
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("literals", ["cargo", "xtask", "check-literals"]),
        (
            "test-quality",
            [
                "cargo",
                "xtask",
                "check-test-quality",
                "--report",
                *SOURCES,
            ],
        ),
    ]:
        checked(command, RAW / f"{name}.log")


def run_phase(phase):
    for mode in ("ordinary", "accelerated"):
        checked(
            [
                "pwsh",
                "-NoProfile",
                "-File",
                str(HERE / "run-ab.ps1"),
                "-Mode",
                mode,
                "-Phase",
                phase,
            ],
            RAW / f"{mode}-{phase}.log",
        )
    if phase == "proof":
        # Reference discipline: the four proof slices must agree with each
        # other and with the preserved job reference before any timing runs.
        proofs = {}
        for mode in ("ordinary", "accelerated"):
            rows = list(
                csv.DictReader(
                    (RAW / f"proof/{mode}/rows.csv").open(encoding="utf-8-sig")
                )
            )
            if len(rows) != 2:
                raise RuntimeError(f"proof population: {mode}")
            proofs[mode] = {row["arm"]: row["sha256"] for row in rows}
            if len(set(proofs[mode].values())) != 1:
                raise RuntimeError(f"proof mismatch: {mode}")
        shas = {mode: next(iter(v.values())) for mode, v in proofs.items()}
        if (
            len(set(shas.values())) != 1
            or next(iter(shas.values())) != REFERENCE_GCODE_SHA
        ):
            raise RuntimeError(f"reference drift before timing: {shas}")
        print(f"proofs agree with the preserved reference: {shas}", flush=True)


if __name__ == "__main__":
    commands = {
        "prepare": prepare,
        "build-candidates": build_candidates,
        "tests": tests,
        "gates": gates,
        "proof": lambda: run_phase("proof"),
        "measure": lambda: run_phase("measure"),
        "stage-ordinary": lambda: stage("ordinary"),
        "stage-accelerated": lambda: stage("accelerated"),
    }
    if len(sys.argv) == 2 and sys.argv[1] in commands:
        commands[sys.argv[1]]()
    else:
        raise RuntimeError(f"expected one of {sorted(commands)}")
