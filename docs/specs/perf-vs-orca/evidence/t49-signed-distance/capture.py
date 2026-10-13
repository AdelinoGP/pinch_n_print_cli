"""One-shot diagnostic capture/reduction, never an acceptance timing runner."""

import collections
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t49-signed-distance"
CONFIG = HERE.parent / "t44-clip-universe-hoist/benchy-arachne.json"
MODEL = REPO / "tmp/3dbenchy.stl"
MODULE = "com.core.arachne-perimeters"
SOURCES = [
    "crates/slicer-core/src/perimeter_spatial.rs",
    "crates/slicer-core/src/perimeter_utils.rs",
    "crates/slicer-core/src/profile.rs",
    "modules/core-modules/arachne-perimeters/src/lib.rs",
]


def sha(path):
    return hashlib.file_digest(Path(path).open("rb"), "sha256").hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def inventory(root):
    return {
        p.relative_to(root).as_posix(): sha(p)
        for p in sorted(root.rglob("*"))
        if p.is_file()
    }


def events(path):
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if isinstance(e, dict) and "event" in e:
            yield e


def checked(command, log):
    with log.open("w", encoding="utf-8") as f:
        result = subprocess.run(command, cwd=REPO, stdout=f, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f"exit {result.returncode}: {command}; see {log}")
    return result.returncode


def prepare():
    if RAW.exists():
        raise RuntimeError(f"one-shot guard: {RAW}")
    if not MODEL.is_file() or not CONFIG.is_file():
        raise RuntimeError("required current input absent; no historical substitution")
    RAW.mkdir(parents=True)
    identity = {
        "head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=REPO, text=True
        ).strip(),
        "model_sha256": sha(MODEL),
        "config_sha256": sha(CONFIG),
        "config": json.loads(CONFIG.read_text()),
        "threads": 12,
        "compiler": subprocess.check_output(["rustc", "-vV"], text=True),
        "freshness": {},
        "source_sha256": {},
        "snapshots": {},
    }
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        identity["freshness"][mode] = {
            "command": command,
            "exit_code": checked(command, RAW / f"{mode}-freshness.log"),
        }
        root = REPO / (
            "target/dist/developer"
            if mode == "ordinary"
            else "target/dist-accelerated/developer"
        )
        dest = RAW / f"{mode}-baseline"
        shutil.copytree(root, dest)
        identity["snapshots"][mode] = inventory(dest)
    files = (
        subprocess.check_output(["git", "ls-files", "-z"], cwd=REPO)
        .decode()
        .split("\0")
    )
    for name in files:
        if name and (
            name.startswith(("crates/", "modules/", "xtask/", "dist/"))
            or name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]
        ):
            path = REPO / name
            if path.is_file():
                identity["source_sha256"][name] = sha(path)
    for name in SOURCES:
        dest = RAW / "source-before" / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / name, dest)
    write(RAW / "identity.json", identity)
    write(HERE / "identity.json", identity)
    print(
        "Inputs, clean mode-specific freshness exits, current source and baseline snapshots recorded."
    )


def capture(mode, phase):
    dest = RAW / mode / phase
    if dest.exists():
        raise RuntimeError(f"one-shot guard: {dest}")
    dest.mkdir(parents=True)
    snapshot = RAW / f"{mode}-{phase}"
    if phase in ["baseline", "baseline-profile"]:
        snapshot = RAW / f"{mode}-baseline"
    else:
        root = REPO / (
            "target/dist/developer"
            if mode == "ordinary"
            else "target/dist-accelerated/developer"
        )
        # Diagnostic host supplies scope names; isolate guest changes to Arachne.
        shutil.copytree(RAW / f"{mode}-baseline", snapshot)
        shutil.copy2(root / "pnp_cli.exe", snapshot / "pnp_cli.exe")
        shutil.copy2(
            root / "modules/arachne-perimeters/arachne-perimeters.wasm",
            snapshot / "modules/arachne-perimeters/arachne-perimeters.wasm",
        )
    command = [
        str(snapshot / "pnp_cli.exe"),
        "slice",
        "--model",
        str(MODEL),
        "--config",
        str(CONFIG),
        "--output",
        str(dest / "output.gcode"),
        "--module-dir",
        str(snapshot / "modules"),
        "--no-default-module-paths",
        "--no-integrated-modules",
    ]
    if phase in ["probe", "baseline-profile"]:
        command += ["--profile", "--instrument-stderr"]
    with (
        (dest / "stderr.jsonl").open("w") as err,
        (dest / "stdout.txt").open("w") as out,
    ):
        result = subprocess.run(
            command,
            cwd=REPO,
            env={**os.environ, "RAYON_NUM_THREADS": "12"},
            stdout=out,
            stderr=err,
            timeout=600,
        )
    ev = list(events(dest / "stderr.jsonl"))
    completed = [e for e in ev if e["event"] == "slice_complete"]
    if result.returncode or len(completed) != 1:
        raise RuntimeError(f"failed/incomplete {mode} {phase}")
    output = dest / "output.gcode"
    lines = output.read_text().splitlines()
    disclosure = {
        "mode": mode,
        "phase": phase,
        "command": command,
        "exit_code": result.returncode,
        "sha256": sha(output),
        "bytes": output.stat().st_size,
        "type_counts": dict(
            sorted(
                collections.Counter(
                    l[6:] for l in lines if l.startswith(";TYPE:")
                ).items()
            )
        ),
        "generator_markers": [l for l in lines if "wall_generator" in l],
        "completion": completed[0],
        "slice_stats": [e for e in ev if e["event"] == "slice_stats"],
        "module_errors": [e for e in ev if e["event"] == "module_error"],
        "open_loop_warnings": sum(
            "is not closed" in l
            for l in (dest / "stderr.jsonl").read_text().splitlines()
        ),
        "inventory": inventory(snapshot),
    }
    if (
        disclosure["module_errors"]
        or completed[0].get("degraded")
        or completed[0].get("status") != "ok"
    ):
        raise RuntimeError(f"unclean output: {completed[0]}")
    write(dest / "disclosure.json", disclosure)
    write(HERE / f"{mode}-{phase}-output.json", disclosure)
    if phase in ["probe", "baseline-profile"]:
        baseline = json.loads((RAW / mode / "baseline/disclosure.json").read_text())
        if any(
            disclosure[k] != baseline[k]
            for k in [
                "sha256",
                "bytes",
                "type_counts",
                "generator_markers",
                "open_loop_warnings",
            ]
        ):
            raise RuntimeError(
                "probe output differs from current reference; do not interpret costs"
            )
        reduced = [
            e
            for e in ev
            if e["event"] in ["profile_summary", "slice_stats", "slice_complete"]
            or (
                e["event"] == "module_log"
                and e.get("module_id") == MODULE
                and "T49-COUNTS" in e.get("message", "")
            )
        ]
        capture_name = (
            f"{mode}-capture.jsonl"
            if phase == "probe"
            else f"{mode}-baseline-profile-capture.jsonl"
        )
        (HERE / capture_name).write_text("".join(json.dumps(e) + "\n" for e in reduced))
    print(
        f"{mode} {phase}: output SHA {disclosure['sha256']}, {disclosure['bytes']} bytes, clean completion"
    )


def restore_check():
    identity = json.loads((RAW / "identity.json").read_text())
    source_checks = {
        name: sha(REPO / name) == digest
        for name, digest in identity["source_sha256"].items()
    }
    assert all(source_checks.values()), "source identity changed"
    restored = {
        "source_identity_matches_before": True,
        "source_file_count_checked": len(source_checks),
        "modes": {},
    }
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        exit_code = checked(command, RAW / f"{mode}-restored-freshness.log")
        root = REPO / (
            "target/dist/developer"
            if mode == "ordinary"
            else "target/dist-accelerated/developer"
        )
        current = inventory(root)
        assert current == identity["snapshots"][mode], (
            "active distribution not byte-restored"
        )
        restored["modes"][mode] = {
            "freshness_command": command,
            "freshness_exit_code": exit_code,
            "inventory_matches_before": True,
            "inventory": current,
        }
    write(HERE / "restoration.json", restored)
    print(
        "All recorded source inputs unchanged; both freshness exits 0; active distributions byte-restored."
    )


def reduce_existing():
    """Rebuild reduced files from saved raw captures without executing a slicer."""
    shutil.copy2(RAW / "identity.json", HERE / "identity.json")
    for mode in ["ordinary", "accelerated"]:
        for phase in ["baseline", "probe", "baseline-profile"]:
            dest = RAW / mode / phase
            shutil.copy2(dest / "disclosure.json", HERE / f"{mode}-{phase}-output.json")
            if phase == "baseline":
                continue
            reduced = [
                e
                for e in events(dest / "stderr.jsonl")
                if e["event"] in ["profile_summary", "slice_stats", "slice_complete"]
                or (
                    e["event"] == "module_log"
                    and e.get("module_id") == MODULE
                    and "T49-COUNTS" in e.get("message", "")
                )
            ]
            name = (
                f"{mode}-capture.jsonl"
                if phase == "probe"
                else f"{mode}-baseline-profile-capture.jsonl"
            )
            (HERE / name).write_text("".join(json.dumps(e) + "\n" for e in reduced))
    print("Reduced existing raw captures; no builds or slices.")


if __name__ == "__main__":
    if sys.argv[1] == "prepare":
        prepare()
    elif sys.argv[1] == "restore-check":
        restore_check()
    elif sys.argv[1] == "reduce-existing":
        reduce_existing()
    else:
        capture(sys.argv[1], sys.argv[2])
