"""Preserve current snapshots, stage one guest, and restore experiment artifacts."""

import hashlib
import csv
import difflib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t51-winding-cache"
SOURCES = [
    "crates/slicer-core/src/perimeter_spatial.rs",
    "crates/slicer-core/tests/perimeter_spatial_tdd.rs",
]
GUEST = "modules/arachne-perimeters/arachne-perimeters.wasm"


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
    return result.returncode


def prepare():
    if RAW.exists():
        raise RuntimeError(f"one-shot guard: {RAW}")
    previous = json.loads(
        (HERE.parent / "t49-signed-distance/identity.json").read_text()
    )
    for name, sha in previous["source_sha256"].items():
        if not (REPO / name).is_file() or digest(REPO / name) != sha:
            raise RuntimeError(f"baseline source provenance mismatch: {name}")
    model = REPO / "tmp/3dbenchy.stl"
    config = HERE.parent / "t44-clip-universe-hoist/benchy-arachne.json"
    if (
        digest(model) != previous["model_sha256"]
        or digest(config) != previous["config_sha256"]
    ):
        raise RuntimeError("input identity differs from authorized attribution job")
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    if compiler != previous["compiler"]:
        raise RuntimeError("baseline compiler provenance mismatch")
    RAW.mkdir(parents=True)
    (RAW / "snapshots").mkdir()
    identity = {
        "baseline_head": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=REPO, text=True
        ).strip(),
        "model_sha256": digest(model),
        "config_sha256": digest(config),
        "compiler": compiler,
        "source_sha256": previous["source_sha256"],
        "baseline_provenance": "current sources/compiler and active distributions reverified against t49 restored baseline",
        "freshness": {},
        "snapshots": {},
    }
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        identity["freshness"][mode] = {
            "command": command,
            "exit_code": checked(command, RAW / f"{mode}-preflight.log"),
        }
        current = inventory(dist(mode))
        if current != previous["snapshots"][mode]:
            raise RuntimeError(f"active snapshot baseline provenance mismatch: {mode}")
        dest = RAW / f"snapshots/{mode}-baseline"
        shutil.copytree(dist(mode), dest)
        identity["snapshots"][mode] = inventory(dest)
    for name in SOURCES:
        dest = RAW / "source-before" / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / name, dest)
    (RAW / "snapshots/head.txt").write_text(identity["baseline_head"] + "\n")
    write(HERE / "identity.json", identity)
    write(RAW / "identity.json", identity)
    print("Current baseline snapshots and source copies preserved.", flush=True)


def stage(mode):
    baseline = RAW / f"snapshots/{mode}-baseline"
    candidate = RAW / f"snapshots/{mode}-candidate"
    if candidate.exists():
        raise RuntimeError(f"one-shot guard: {candidate}")
    shutil.copytree(baseline, candidate)
    # build-guests stages mode-distinct components; never substitute the other mode.
    source = REPO / (
        "modules/core-modules/arachne-perimeters/arachne-perimeters.wasm"
        if mode == "ordinary"
        else "target/guests-accelerated/artifacts/modules/core-modules/arachne-perimeters/arachne-perimeters.wasm"
    )
    shutil.copy2(source, candidate / GUEST)
    arms_path = RAW / "snapshots/arm-inventory.json"
    arms = json.loads(arms_path.read_text()) if arms_path.exists() else {}
    a, b = inventory(baseline), inventory(candidate)
    changes = [k for k in a if a[k] != b[k]]
    if a.keys() != b.keys() or changes != [GUEST]:
        raise RuntimeError(f"non-isolated candidate: {mode}: {changes}")
    arms[mode] = {"baseline": a, "candidate": b, "changed": changes}
    write(arms_path, arms)
    print(f"Staged isolated {mode} Arachne guest.")


def restore_distributions():
    restored = {}
    for mode in ("ordinary", "accelerated"):
        baseline = RAW / f"snapshots/{mode}-baseline"
        expected = inventory(baseline)
        if inventory(dist(mode)).keys() != expected.keys():
            raise RuntimeError(f"unexpected active distribution file set: {mode}")
        shutil.copytree(baseline, dist(mode), dirs_exist_ok=True)
        current = inventory(dist(mode))
        if current != expected:
            raise RuntimeError(f"restoration mismatch: {mode}")
        restored[mode] = {"inventory": current, "inventory_matches_baseline": True}
    write(HERE / "restoration.json", restored)
    print("Both active distributions byte-restored from their pre-edit snapshots.")


def build_candidates():
    checked(["cargo", "xtask", "build-guests"], RAW / "ordinary-candidate-build.log")
    checked(
        ["cargo", "xtask", "build-guests", "--check"],
        RAW / "ordinary-candidate-freshness.log",
    )
    stage("ordinary")
    checked(
        [
            "cargo",
            "xtask",
            "test",
            "--accelerated",
            "--summary",
            "-p",
            "slicer-core",
            "--features",
            "host-algos,perimeter-spatial-test-support",
            "--test",
            "perimeter_spatial_tdd",
        ],
        RAW / "accelerated-candidate-tests.log",
    )
    shutil.copy2(
        REPO / "target/test-output.log", HERE / "accelerated-exactness-tests.log"
    )
    checked(
        ["cargo", "xtask", "build-guests", "--accelerated", "--check"],
        RAW / "accelerated-candidate-freshness.log",
    )
    stage("accelerated")


def freeze_candidate():
    patch = subprocess.check_output(
        ["git", "--no-pager", "diff", "HEAD", "--", *SOURCES], cwd=REPO
    )
    (HERE / "candidate.patch").write_bytes(patch)
    for name in SOURCES + [
        "crates/slicer-core/src/profile.rs",
        "modules/core-modules/arachne-perimeters/src/lib.rs",
    ]:
        dest = RAW / "candidate-before-probe" / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / name, dest)
    print("Candidate patch and pre-probe source copies preserved.")


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
                "--features",
                "slicer-core/perimeter-spatial-test-support",
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
                "crates/slicer-core/src/perimeter_spatial.rs",
                "crates/slicer-core/tests/perimeter_spatial_tdd.rs",
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


def guest_source(mode):
    return REPO / (
        "modules/core-modules/arachne-perimeters/arachne-perimeters.wasm"
        if mode == "ordinary"
        else "target/guests-accelerated/artifacts/modules/core-modules/arachne-perimeters/arachne-perimeters.wasm"
    )


def capture_profile(mode, phase):
    dest = RAW / "accounting" / mode / phase
    if dest.exists():
        raise RuntimeError(f"one-shot guard: {dest}")
    dest.mkdir(parents=True)
    snapshot = RAW / f"snapshots/{mode}-{'probe' if phase == 'probe' else 'candidate'}"
    command = [
        str(snapshot / "pnp_cli.exe"),
        "slice",
        "--model",
        str(REPO / "tmp/3dbenchy.stl"),
        "--config",
        str(HERE.parent / "t44-clip-universe-hoist/benchy-arachne.json"),
        "--output",
        str(dest / "output.gcode"),
        "--module-dir",
        str(snapshot / "modules"),
        "--no-default-module-paths",
        "--no-integrated-modules",
        "--profile",
        "--instrument-stderr",
    ]
    with (
        (dest / "stderr.jsonl").open("w") as err,
        (dest / "stdout.log").open("w") as out,
    ):
        result = subprocess.run(
            command,
            cwd=REPO,
            env={**os.environ, "RAYON_NUM_THREADS": "12"},
            stderr=err,
            stdout=out,
            timeout=600,
        )
    events = []
    for line in (dest / "stderr.jsonl").read_text().splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if isinstance(event, dict) and "event" in event:
            events.append(event)
    completion = [e for e in events if e["event"] == "slice_complete"]
    if (
        result.returncode
        or len(completion) != 1
        or completion[0].get("status") != "ok"
        or completion[0].get("degraded")
    ):
        raise RuntimeError(f"failed diagnostic output: {mode}/{phase}")
    with (RAW / f"proof/{mode}/rows.csv").open(encoding="utf-8-sig") as stream:
        reference = next(csv.DictReader(stream))
    if digest(dest / "output.gcode") != reference["sha256"]:
        raise RuntimeError(f"diagnostic output drift: {mode}/{phase}")
    reduced = [
        e
        for e in events
        if e["event"]
        in ("profile_summary", "slice_stats", "slice_complete", "module_error")
        or (e["event"] == "module_log" and "PERF-T51-COUNTS" in e.get("message", ""))
    ]
    write(
        HERE / f"{mode}-{phase}-profile.json",
        {
            "command": command,
            "exit_code": result.returncode,
            "sha256": digest(dest / "output.gcode"),
            "bytes": (dest / "output.gcode").stat().st_size,
            "snapshot": inventory(snapshot),
            "events": reduced,
        },
    )
    print(f"PASS: {mode}/{phase} diagnostic output matches current proof", flush=True)


def diagnostic_accounting():
    patch = []
    probe_sources = [
        SOURCES[0],
        "crates/slicer-core/src/profile.rs",
        "modules/core-modules/arachne-perimeters/src/lib.rs",
    ]
    for name in probe_sources:
        before = (
            (RAW / "candidate-before-probe" / name)
            .read_text()
            .splitlines(keepends=True)
        )
        after = (REPO / name).read_text().splitlines(keepends=True)
        patch.extend(
            difflib.unified_diff(
                before, after, fromfile=f"a/{name}", tofile=f"b/{name}"
            )
        )
    (HERE / "probe.patch").write_text("".join(patch), encoding="utf-8")
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        checked(
            ["cargo", "xtask", "build-guests", *flags], RAW / f"{mode}-probe-build.log"
        )
        checked(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            RAW / f"{mode}-probe-freshness.log",
        )
        probe = RAW / f"snapshots/{mode}-probe"
        if probe.exists():
            raise RuntimeError(f"one-shot guard: {probe}")
        shutil.copytree(RAW / f"snapshots/{mode}-candidate", probe)
        shutil.copy2(guest_source(mode), probe / GUEST)
        capture_profile(mode, "control")
        capture_profile(mode, "probe")


def restore_candidate_sources():
    for name in [
        SOURCES[0],
        "crates/slicer-core/src/profile.rs",
        "modules/core-modules/arachne-perimeters/src/lib.rs",
    ]:
        if digest(REPO / name) != digest(RAW / "candidate-before-probe" / name):
            raise RuntimeError(
                f"remove diagnostic-only source changes before restoring artifacts: {name}"
            )
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        checked(
            ["cargo", "xtask", "build-guests", *flags],
            RAW / f"{mode}-post-probe-build.log",
        )
        checked(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            RAW / f"{mode}-post-probe-freshness.log",
        )
        expected = digest(RAW / f"snapshots/{mode}-candidate" / GUEST)
        if digest(guest_source(mode)) != expected:
            raise RuntimeError(
                f"candidate artifact differs after probe restoration: {mode}"
            )
    print(
        "Diagnostic sources removed; probe-free candidate guest identities reverified."
    )


def authorized_retry():
    """One human-authorized repeat; retain the first campaign and identical arms."""
    retry = RAW / "retry-1"
    if retry.exists():
        raise RuntimeError(f"one-shot guard: {retry}")
    arms = json.loads(
        (RAW / "snapshots/arm-inventory.json").read_text(encoding="utf-8")
    )
    identity = json.loads((HERE / "identity.json").read_text(encoding="utf-8"))
    # Source, toolchain and snapshot checks finish before any timing subprocess.
    for name in SOURCES + [
        "crates/slicer-core/src/profile.rs",
        "modules/core-modules/arachne-perimeters/src/lib.rs",
    ]:
        if digest(REPO / name) != digest(RAW / "candidate-before-probe" / name):
            raise RuntimeError(f"candidate source changed since first campaign: {name}")
    if subprocess.check_output(["rustc", "-vV"], text=True) != identity["compiler"]:
        raise RuntimeError("candidate compiler identity changed")
    for mode in ("ordinary", "accelerated"):
        for arm in ("baseline", "candidate"):
            if inventory(RAW / f"snapshots/{mode}-{arm}") != arms[mode][arm]:
                raise RuntimeError(f"snapshot changed: {mode}/{arm}")
        if inventory(dist(mode)) != arms[mode]["baseline"]:
            raise RuntimeError(f"active distribution not restored: {mode}")
        if digest(guest_source(mode)) != arms[mode]["candidate"][GUEST]:
            raise RuntimeError(f"shared candidate artifact changed: {mode}")
    retry.mkdir()
    authorization = {
        "human_request": "Try one more run",
        "scope": "one additional complete campaign in both modes; no pooling, changed thresholds, source changes or further retries",
        "candidate_patch_sha256": digest(HERE / "candidate.patch"),
        "protocol_sha256": digest(HERE / "protocol.json"),
        "preserved_initial_evidence_sha256": digest(HERE / "evidence.json"),
        "preserved_initial_summary_sha256": digest(HERE / "summary.json"),
        "source_and_snapshot_identity_checks": "passed",
        "freshness": {},
    }
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        authorization["freshness"][mode] = {
            "command": command,
            "exit_code": checked(command, retry / f"{mode}-preflight.log"),
        }
    write(HERE / "retry-1-authorization.json", authorization)
    write(retry / "authorization.json", authorization)
    for phase in ("proof", "measure"):
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
                    "-Campaign",
                    "retry-1",
                ],
                retry / f"{mode}-{phase}.log",
            )
    print(
        "Completed the one authorized repeat; initial evidence untouched.", flush=True
    )


def cleanup_drop():
    """Human-approved DROP: retire candidate source/tests, rebuild, verify."""
    cleanup = RAW / "cleanup"
    if cleanup.exists():
        raise RuntimeError(f"one-shot guard: {cleanup}")
    # Protect the candidate first: it must still be exactly the preserved patch, so
    # cleanup can never discard work that is not already frozen and re-appliable.
    current = subprocess.run(
        ["git", "--no-pager", "diff", "HEAD", "--", *SOURCES],
        cwd=REPO,
        capture_output=True,
        check=True,
    ).stdout
    if current != (HERE / "candidate.patch").read_bytes():
        raise RuntimeError("working tree differs from the preserved candidate patch")
    for name in SOURCES:
        head = subprocess.run(
            ["git", "show", f"HEAD:{name}"], cwd=REPO, capture_output=True, check=True
        ).stdout
        if hashlib.sha256(head).hexdigest() != digest(RAW / "source-before" / name):
            raise RuntimeError(
                f"HEAD differs from preserved pre-experiment source: {name}"
            )
    cleanup.mkdir()
    # Retire candidate source/tests together to the preserved pre-experiment tree.
    for name in SOURCES:
        shutil.copy2(RAW / "source-before" / name, REPO / name)
    for name in SOURCES:
        head = subprocess.run(
            ["git", "show", f"HEAD:{name}"], cwd=REPO, capture_output=True, check=True
        ).stdout
        if hashlib.sha256(head).hexdigest() != digest(REPO / name):
            raise RuntimeError(f"reverted source differs from HEAD: {name}")
    # Rebuild the shared guest artifacts from the restored tree.
    for mode, flags in [("ordinary", []), ("accelerated", ["--accelerated"])]:
        checked(
            ["cargo", "xtask", "build-guests", *flags], cleanup / f"{mode}-rebuild.log"
        )
        checked(
            ["cargo", "xtask", "build-guests", *flags, "--check"],
            cleanup / f"{mode}-freshness.log",
        )
        rebuilt = digest(guest_source(mode))
        baseline = digest(RAW / f"snapshots/{mode}-baseline" / GUEST)
        if rebuilt != baseline:
            raise RuntimeError(
                f"rebuilt {mode} guest differs from archived baseline: {rebuilt} != {baseline}"
            )
    # Prove the restored original suite still compiles and passes.
    restored_test = cleanup / "restored-suite.log"
    with (REPO / "target/test-output.log").open("w", encoding="utf-8") as stream:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "-p",
                "slicer-core",
                "--features",
                "host-algos,perimeter-spatial-test-support",
                "--test",
                "perimeter_spatial_tdd",
            ],
            cwd=REPO,
            stdout=stream,
            stderr=subprocess.STDOUT,
        )
    shutil.copy2(REPO / "target/test-output.log", restored_test)
    if result.returncode:
        raise RuntimeError(f"restored original suite failed; see {restored_test}")
    print(f"PASS: restored original suite ({restored_test})", flush=True)
    # The frozen patch must remain re-applicable to the restored tree.
    patch_log = subprocess.run(
        ["git", "apply", "--check", str(HERE / "candidate.patch")],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    (cleanup / "patch-reapplicable.log").write_text(
        f"exit {patch_log.returncode}\n{patch_log.stdout}{patch_log.stderr}",
        encoding="utf-8",
    )
    if patch_log.returncode:
        raise RuntimeError("preserved patch is not re-applicable to the restored tree")
    print("PASS: preserved candidate patch remains re-applicable.", flush=True)
    cleanup_record = {
        "human_decision": "drop",
        "candidate_patch_sha256": digest(HERE / "candidate.patch"),
        "source": {
            name: {
                "restored_sha256": digest(REPO / name),
                "matches_head_and_pre_experiment": True,
            }
            for name in SOURCES
        },
        "rebuilt_guest_sha256": {
            mode: digest(guest_source(mode)) for mode in ("ordinary", "accelerated")
        },
        "rebuild_matches_archived_baseline": True,
        "restored_suite_log": restored_test.name,
        "patch_reapplicable_to_restored_tree": True,
        "active_distribution_restoration": "restoration.json (unchanged)",
        "automatic_commit": False,
        "freshness": {
            mode: {
                "command": [
                    "cargo",
                    "xtask",
                    "build-guests",
                    *([] if mode == "ordinary" else ["--accelerated"]),
                    "--check",
                ],
                "exit_code": 0,
            }
            for mode in ("ordinary", "accelerated")
        },
    }
    write(HERE / "cleanup.json", cleanup_record)
    write(cleanup / "cleanup.json", cleanup_record)
    print(
        "DROP cleanup complete: source, artifacts, patch and gates verified.",
        flush=True,
    )


if __name__ == "__main__":
    if sys.argv[1] == "prepare":
        prepare()
    elif sys.argv[1] == "stage":
        stage(sys.argv[2])
    elif sys.argv[1] == "restore-distributions":
        restore_distributions()
    elif sys.argv[1] == "build-candidates":
        build_candidates()
    elif sys.argv[1] == "gates":
        gates()
    elif sys.argv[1] == "freeze-candidate":
        freeze_candidate()
    elif sys.argv[1] in ("proof", "measure"):
        run_phase(sys.argv[1])
    elif sys.argv[1] == "accounting":
        diagnostic_accounting()
    elif sys.argv[1] == "restore-candidate":
        restore_candidate_sources()
    elif sys.argv[1] == "authorized-retry":
        authorized_retry()
    elif sys.argv[1] == "cleanup-drop":
        cleanup_drop()
    else:
        raise RuntimeError("expected prepare, stage MODE, or restore-distributions")
