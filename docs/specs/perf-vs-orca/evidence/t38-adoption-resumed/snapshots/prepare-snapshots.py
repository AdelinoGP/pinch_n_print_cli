"""One authorized t38 campaign snapshot preparation (ordinary + accelerated).

Builds complete `developer` edition snapshots with the controlled xtask entry
points, verifies mode/policy metadata from the actual identity files, copies
both staged snapshots into the durable namespace, and runs read-only
module-diagnosis probes. It never slices a model, runs a test, or measures
timing. One attempt only: any failure stops the helper; it never repairs or
retries.

Usage: python prepare-snapshots.py
"""

import hashlib
import json
import os
import re
import runpy
import shutil
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

HELPER = Path(__file__).resolve()
EVIDENCE = HELPER.parent
TEST_SUPPORT_FEATURE = "perimeter-spatial-test-support"


def find_root(start):
    probe = start
    while True:
        if (
            (probe / "Cargo.toml").is_file()
            and (probe / "xtask").is_dir()
            and (probe / "docs/23_controlled_perimeter_builds.md").is_file()
        ):
            return probe
        if probe.parent == probe:
            raise RuntimeError("workspace root not found above helper")
        probe = probe.parent


ROOT = find_root(EVIDENCE)
T41 = ROOT / "docs/specs/perf-vs-orca/evidence/t41-reference-repreparation"
T41_ATTEMPT = T41 / "attempt-1"
T41_DISCOVERY = (
    ROOT
    / "docs/specs/perf-vs-orca/evidence/t38-adoption-resumed/runner/ordinary-runner-discovery.stdout.log"
)
DURABLE = ROOT / ".local-artifacts/perimeter-reference-preparation"
CAMPAIGN_INPUTS = (
    "resources/perimeter-acceptance/run_bench.ps1",
    "resources/perimeter-acceptance/run-acceptance.ps1",
    "resources/perimeter-acceptance/validate_measurement.ps1",
    "resources/perimeter-acceptance/test-status-roundtrip.ps1",
    "xtask/src/rustc_driver.rs",
    "docs/23_controlled_perimeter_builds.md",
)

STATE = {
    "status": "preparing",
    "purpose": (
        "complete ordinary + accelerated developer production campaign "
        "snapshots (builds and read-only module diagnosis only; no slices, "
        "tests or timing)"
    ),
    "commands": [],
    "freshness_gates": {},
    "gaps": [],
}

LOG_DIR = None
WORK_ROOT = None


def utc():
    return datetime.now(timezone.utc).isoformat()


def norm(text):
    return text.replace("\r\n", "\n").replace("\r", "\n")


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def inventory(directory):
    directory = Path(directory)
    return {
        path.relative_to(directory).as_posix(): digest(path)
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def write_json(path, value):
    Path(path).write_text(
        json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def save_state():
    target = (
        (LOG_DIR / "snapshot-state.json")
        if LOG_DIR is not None
        else (EVIDENCE / "snapshot-state-blocked.json")
    )
    write_json(target, STATE)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def run_cmd(name, argv, env=None, expect=0):
    argv = [str(arg) for arg in argv]
    print(f"RUN {name}: {' '.join(argv)}", flush=True)
    result = subprocess.run(argv, cwd=str(ROOT), capture_output=True, env=env)
    (LOG_DIR / f"{name}.stdout.log").write_bytes(result.stdout)
    (LOG_DIR / f"{name}.stderr.log").write_bytes(result.stderr)
    STATE["commands"].append(
        {
            "name": name,
            "argv": argv,
            "exit_code": result.returncode,
            "stdout_log": f"logs/{name}.stdout.log",
            "stderr_log": f"logs/{name}.stderr.log",
        }
    )
    save_state()
    if expect is not None and result.returncode != expect:
        raise RuntimeError(
            f"{name} exited {result.returncode}, expected {expect}; "
            f"see logs/{name}.stdout.log and logs/{name}.stderr.log"
        )
    return result


def check_or_rebuild(label, check_argv, rebuild_argv):
    """Requested-mode freshness gate.

    Exit 0 passes. Exit 1 with STALE: lines is the one documented repair path
    (rebuild only the requested mode exactly once, then recheck). Lock
    convergence and infrastructure errors stop the helper.
    """
    result = run_cmd(f"build-guests-{label}-check", check_argv, expect=None)
    record = {"check_exit": result.returncode}
    if result.returncode == 0:
        record["rebuild_performed"] = False
        STATE["freshness_gates"][label] = record
        save_state()
        return record
    text = norm(result.stdout.decode("utf-8", "replace")) + norm(
        result.stderr.decode("utf-8", "replace")
    )
    if "LOCK-DIVERGENCE:" in text:
        raise RuntimeError(
            f"{label} freshness check reported lock convergence "
            f"(exit {result.returncode}); policy stops"
        )
    if result.returncode == 3:
        raise RuntimeError(
            f"{label} freshness check exited 3 (infrastructure error); policy stops"
        )
    if result.returncode == 1 and "STALE:" in text:
        rebuild = run_cmd(f"build-guests-{label}-rebuild", rebuild_argv)
        require(
            rebuild.returncode == 0,
            f"{label} requested-mode stale rebuild exited {rebuild.returncode}",
        )
        recheck = run_cmd(f"build-guests-{label}-recheck", check_argv)
        record.update(
            {
                "rebuild_performed": True,
                "rebuild_exit": rebuild.returncode,
                "recheck_exit": recheck.returncode,
            }
        )
        require(
            recheck.returncode == 0,
            f"{label} recheck did not reach exit 0 after the documented stale rebuild",
        )
        STATE["freshness_gates"][label] = record
        save_state()
        return record
    raise RuntimeError(
        f"{label} freshness check exited {result.returncode} without a STALE: line; "
        "policy stops"
    )


def parse_plan(text):
    parsed = {}
    for line in norm(text).splitlines():
        if "\t" in line:
            key, value = line.split("\t", 1)
            parsed.setdefault(key, []).append(value)
    return parsed


def parse_search_roots(stderr_text):
    roots = []
    lines = norm(stderr_text).splitlines()
    for index, line in enumerate(lines):
        match = re.match(r"^pnp_cli: module search roots \((\d+)\):$", line.strip())
        if match:
            count = int(match.group(1))
            for follow in lines[index + 1 : index + 1 + count]:
                entry = re.match(r"^\s*\[(\d+)\]\s+(.*)$", follow)
                if entry:
                    roots.append(entry.group(2).strip())
    return roots


def run_diagnose(label, snapshot, extra_flags, expected_ids):
    argv = [
        snapshot / "pnp_cli.exe",
        "module",
        "diagnose",
        "--module-dir",
        snapshot / "modules",
        *extra_flags,
    ]
    env = dict(os.environ, SLICER_DEBUG_PATHS="1")
    result = run_cmd(f"diagnose-{label}", argv, env=env)
    require(result.returncode == 0, f"diagnose {label} exited {result.returncode}")
    report = json.loads(result.stdout.decode("utf-8-sig"))
    require(report.get("pass") is True, f"diagnose {label} did not pass")
    modules = report.get("modules") or []
    require(modules, f"diagnose {label} loaded no modules")
    require(
        all(module["provenance"] == "external" for module in modules),
        f"diagnose {label} loaded a non-external module",
    )
    ids = sorted(module["id"] for module in modules)
    require(
        ids == expected_ids,
        f"diagnose {label} module set differs from the frozen t41 discovery set",
    )
    roots = parse_search_roots(result.stderr.decode("utf-8", "replace"))
    require(
        len(roots) == 1,
        f"diagnose {label} reported {len(roots)} search roots, expected exactly 1",
    )
    require(
        os.path.normcase(roots[0]) == os.path.normcase(str(snapshot / "modules")),
        f"diagnose {label} search root {roots[0]!r} is not the snapshot modules dir",
    )
    return {
        "exit_code": 0,
        "modules_loaded": report["modules_loaded"],
        "stages": report["stages"],
        "diagnostics": len(report["diagnostics"]),
        "module_ids": ids,
        "search_roots": roots,
        "stdout_log": f"logs/diagnose-{label}.stdout.log",
        "stderr_log": f"logs/diagnose-{label}.stderr.log",
    }


def contains_bytes(path, needle):
    overlap = b""
    with Path(path).open("rb") as stream:
        while chunk := stream.read(1 << 20):
            window = overlap + chunk
            if needle in window:
                return True
            overlap = window[-(len(needle) - 1) :]
    return False


def main():
    global LOG_DIR, WORK_ROOT

    require(not sys.flags.optimize, "Python assertions must be enabled")
    require(
        not (EVIDENCE / "snapshot-manifest.json").exists(),
        "snapshot preparation already produced a manifest; this helper runs once",
    )

    bad = sorted(key for key in os.environ if key.startswith(("PNP_", "SLICER_")))
    for key in (
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_TARGET",
    ):
        if os.environ.get(key):
            bad.append(key)
    require(
        not bad,
        "unexpected Rust/build/module environment: " + ", ".join(sorted(set(bad))),
    )
    STATE["environment_info"] = {
        key: os.environ.get(key)
        for key in ("RUSTUP_TOOLCHAIN", "CARGO_HOME", "RUSTUP_HOME", "PYTHONUTF8")
    }

    stamp = datetime.now(timezone.utc).strftime("t38-campaign-%Y%m%dT%H%M%SZ")
    WORK_ROOT = DURABLE / stamp
    WORK_ROOT.mkdir(parents=True, exist_ok=False)
    LOG_DIR = WORK_ROOT / "logs"
    LOG_DIR.mkdir()
    require(
        WORK_ROOT.is_relative_to(ROOT / ".local-artifacts"),
        "durable root escaped .local-artifacts",
    )
    require(
        not WORK_ROOT.is_relative_to(ROOT / "target"),
        "durable root must not sit under target/",
    )
    STATE.update(
        {
            "started_utc": utc(),
            "durable_root": str(WORK_ROOT),
            "durable_namespace": str(DURABLE),
            "evidence_dir": str(EVIDENCE),
        }
    )
    save_state()

    t41_manifest_path = T41_ATTEMPT / "frozen-manifest.json"
    t41_manifest = json.loads(t41_manifest_path.read_text(encoding="utf-8"))
    t41_manifest_sha = digest(t41_manifest_path)
    require(
        t41_manifest_sha
        == (T41_ATTEMPT / "frozen-manifest.sha256").read_text(encoding="utf-8").strip(),
        "t41 manifest sidecar disagrees with the manifest file",
    )

    # First: the read-only t41 identity gate, saved with its actual exit code.
    before = run_cmd(
        "t41-freeze-verification-before",
        [sys.executable, T41 / "verify-freeze.py", T41_ATTEMPT],
    )
    before_summary = json.loads(before.stdout.decode("utf-8-sig"))
    require(
        before_summary.get("status") == "PASS",
        "t41 verifier summary is not PASS",
    )
    STATE["t41_gate_before"] = {
        "exit_code": 0,
        "summary_sha256": digest_bytes(before.stdout),
    }
    shutil.copy2(
        LOG_DIR / "t41-freeze-verification-before.stdout.log",
        EVIDENCE / "t41-freeze-verification-before.json",
    )

    # Exact controlled compiler identity and policy constants.
    rustc = run_cmd("rustc-vV", ["rustc", "-vV"])
    identity_text = norm(rustc.stdout.decode("utf-8", "replace"))
    STATE["compiler_identity"] = identity_text
    require(
        identity_text == norm((T41 / "toolchain.log").read_text(encoding="utf-8")),
        "rustc -vV differs from the t41 preflight toolchain log",
    )
    require(
        identity_text
        == norm((T41_ATTEMPT / "snapshot-toolchain.log").read_text(encoding="utf-8")),
        "rustc -vV differs from the t41 snapshot toolchain log",
    )
    fields = {
        line.split(":", 1)[0].strip(): line.split(":", 1)[1].strip()
        for line in identity_text.splitlines()
        if ":" in line
    }
    policy_source_text = (ROOT / "xtask/src/rustc_driver.rs").read_text(
        encoding="utf-8"
    )

    def policy_constant(name):
        match = re.search(rf'const {name}: &str = "([^"]+)"', policy_source_text)
        require(match is not None, f"policy constant {name} not found")
        return match.group(1)

    policy = {
        name: policy_constant(name)
        for name in (
            "ALLOWED_RELEASE",
            "ALLOWED_COMMIT_HASH",
            "ALLOWED_HOST",
            "ALLOWED_LLVM_VERSION",
            "ALLOWED_WASM_TARGET",
            "RESERVED_CFG",
        )
    }
    require(
        fields.get("release") == policy["ALLOWED_RELEASE"], "rustc release vs policy"
    )
    require(
        fields.get("commit-hash") == policy["ALLOWED_COMMIT_HASH"],
        "rustc commit vs policy",
    )
    require(fields.get("host") == policy["ALLOWED_HOST"], "rustc host vs policy")
    require(
        fields.get("LLVM version") == policy["ALLOWED_LLVM_VERSION"],
        "rustc LLVM vs policy",
    )
    require(
        policy["ALLOWED_WASM_TARGET"] == "wasm32-unknown-unknown",
        "policy wasm target changed",
    )
    require(
        policy["RESERVED_CFG"] == "pnp_perimeter_spatial_accelerated",
        "reserved acceleration cfg changed",
    )
    STATE["policy"] = {
        "source_path": "xtask/src/rustc_driver.rs",
        "source_sha256": digest(ROOT / "xtask/src/rustc_driver.rs"),
        "doc_path": "docs/23_controlled_perimeter_builds.md",
        "doc_sha256": digest(ROOT / "docs/23_controlled_perimeter_builds.md"),
        "constants": policy,
    }
    require(
        STATE["policy"]["source_sha256"] == t41_manifest["policy_source_sha256"],
        "controlled-build policy source changed since the t41 freeze",
    )
    require(
        STATE["policy"]["doc_sha256"] == t41_manifest["policy_doc_sha256"],
        "controlled-build policy doc changed since the t41 freeze",
    )

    cargo = run_cmd("cargo-version", ["cargo", "--version"])
    STATE["cargo_version"] = cargo.stdout.decode("utf-8", "replace").strip()
    wasm_tools = run_cmd("wasm-tools-version", ["wasm-tools", "--version"])
    STATE["wasm_tools_version"] = wasm_tools.stdout.decode("utf-8", "replace").strip()

    STATE["campaign_inputs"] = {name: digest(ROOT / name) for name in CAMPAIGN_INPUTS}
    require(
        STATE["campaign_inputs"][
            "resources/perimeter-acceptance/validate_measurement.ps1"
        ]
        == t41_manifest["shared_validator_sha256"],
        "shared measurement validator changed since the t41 freeze",
    )

    # Source build-input identity, before any build.
    prepare_module = runpy.run_path(str(T41 / "prepare.py"))
    source_inputs = prepare_module["source_inputs"]
    sources_before = source_inputs()
    expected_sources = json.loads(
        (T41_ATTEMPT / "source-inputs.json").read_text(encoding="utf-8")
    )
    require(
        sources_before == expected_sources,
        "source build inputs differ from the t41 frozen inventory",
    )
    canonical_sources = digest_bytes(
        json.dumps(sources_before, sort_keys=True).encode("utf-8")
    )
    STATE["source_identity"] = {
        "inputs_count": len(sources_before),
        "inputs_canonical_sha256": canonical_sources,
        "matches_t41_source_inputs": True,
    }

    # Git revision and dirty-worktree evidence.
    revision = run_cmd("git-rev-parse", ["git", "rev-parse", "HEAD"])
    status = run_cmd("git-status", ["git", "status", "--short"])
    diff = run_cmd("git-diff", ["git", "--no-pager", "diff", "--binary", "HEAD"])
    (EVIDENCE / "git-revision.txt").write_text(
        revision.stdout.decode("utf-8", "replace").strip() + "\n", encoding="utf-8"
    )
    (EVIDENCE / "source-worktree-status.txt").write_bytes(status.stdout)
    (EVIDENCE / "source-worktree.diff").write_bytes(diff.stdout)
    STATE["source_identity"].update(
        {
            "revision": revision.stdout.decode("utf-8", "replace").strip(),
            "revision_matches_t41": revision.stdout.decode("utf-8", "replace").strip()
            == t41_manifest["source_revision"],
            "status_sha256": digest_bytes(status.stdout),
            "diff_sha256": digest_bytes(diff.stdout),
        }
    )

    # Target state before builds and preservation of previous dist bundles.
    STATE["target_state_before"] = {
        "dist_developer_present": (ROOT / "target/dist/developer").is_dir(),
        "dist_accelerated_present": (
            ROOT / "target/dist-accelerated/developer"
        ).is_dir(),
        "dist_host_accelerated_present": (
            ROOT / "target/dist-host-accelerated"
        ).is_dir(),
        "release_pnp_cli_sha256": (
            digest(ROOT / "target/release/pnp_cli.exe")
            if (ROOT / "target/release/pnp_cli.exe").is_file()
            else None
        ),
    }
    t41_stems = sorted(
        {
            key.split("/")[1]
            for key in t41_manifest["snapshot_sha256"]
            if key.startswith("modules/")
        }
    )
    require(len(t41_stems) == 24, "t41 snapshot does not name 24 module directories")

    previous_records = {}
    for label, source in (
        ("ordinary", ROOT / "target/dist/developer"),
        ("accelerated", ROOT / "target/dist-accelerated/developer"),
    ):
        record = {"present": source.is_dir()}
        if source.is_dir():
            destination = WORK_ROOT / "preserved-previous-dist" / label
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(source, destination)
            staged_previous = inventory(source)
            require(
                inventory(destination) == staged_previous,
                f"preserved previous {label} dist copy differs from its source",
            )
            record["copied_to"] = str(destination)
            record["inventory_sha256"] = staged_previous
            if label == "ordinary":
                record["equals_t41_frozen_snapshot"] = (
                    staged_previous == t41_manifest["snapshot_sha256"]
                )
        previous_records[label] = record
    STATE["previous_dist"] = previous_records

    # Read-only plans for both modes: the edition flags and the fact that no
    # feature (in particular no test-support feature) is enabled.
    plan_ordinary = run_cmd(
        "dist-plan-ordinary",
        ["cargo", "xtask", "dist", "--plan", "--edition", "developer"],
    )
    plan_accelerated = run_cmd(
        "dist-plan-accelerated",
        ["cargo", "xtask", "dist", "--accelerated", "--plan", "--edition", "developer"],
    )
    plan_ordinary_text = plan_ordinary.stdout.decode("utf-8", "replace")
    plan_accelerated_text = plan_accelerated.stdout.decode("utf-8", "replace")
    parsed_ordinary_plan = parse_plan(plan_ordinary_text)
    parsed_accelerated_plan = parse_plan(plan_accelerated_text)
    for label, parsed in (
        ("ordinary", parsed_ordinary_plan),
        ("accelerated", parsed_accelerated_plan),
    ):
        require(
            parsed.get("edition") == ["developer"],
            f"{label} dist plan edition is not developer",
        )
        require(
            parsed.get("features") == [""],
            f"{label} dist plan enables cargo features",
        )
        require("integrated" not in parsed, f"{label} dist plan integrates modules")
        require(
            TEST_SUPPORT_FEATURE not in plan_ordinary_text
            and TEST_SUPPORT_FEATURE not in plan_accelerated_text,
            "test-support feature appears in a dist plan",
        )
    require(
        parsed_ordinary_plan["out_dir"][0].endswith("dist\\developer"),
        "ordinary dist plan out_dir is not target/dist/developer",
    )
    require(
        "dist-accelerated" in parsed_accelerated_plan["out_dir"][0]
        and parsed_accelerated_plan["out_dir"][0].endswith("developer"),
        "accelerated dist plan out_dir is not target/dist-accelerated/developer",
    )

    # Build the ordinary snapshot.
    run_cmd("dist-ordinary-build", ["cargo", "xtask", "dist", "--edition", "developer"])
    check_or_rebuild(
        "ordinary",
        ["cargo", "xtask", "build-guests", "--check"],
        ["cargo", "xtask", "build-guests"],
    )
    staged_ordinary = ROOT / "target/dist/developer"
    require(staged_ordinary.is_dir(), "ordinary dist dir missing after build")
    durable_ordinary = WORK_ROOT / "ordinary"
    shutil.copytree(staged_ordinary, durable_ordinary)
    ordinary_staged_inventory = inventory(staged_ordinary)
    ordinary_inventory = inventory(durable_ordinary)
    require(
        ordinary_inventory == ordinary_staged_inventory,
        "ordinary snapshot copy inventory differs from the staged dist",
    )
    require(
        ordinary_inventory == t41_manifest["snapshot_sha256"],
        "ordinary snapshot inventory differs from the t41 frozen snapshot",
    )
    require(
        sorted(
            path.name
            for path in (durable_ordinary / "modules").iterdir()
            if path.is_dir()
        )
        == t41_stems,
        "ordinary snapshot module directory set differs from the t41 module set",
    )
    STATE["ordinary_snapshot"] = {
        "root": str(durable_ordinary),
        "exe": str(durable_ordinary / "pnp_cli.exe"),
        "modules_dir": str(durable_ordinary / "modules"),
        "inventory_sha256": ordinary_inventory,
        "matches_t41_frozen_snapshot": True,
    }

    # Build the accelerated snapshot.
    run_cmd(
        "dist-accelerated-build",
        ["cargo", "xtask", "dist", "--accelerated", "--edition", "developer"],
    )
    check_or_rebuild(
        "accelerated",
        ["cargo", "xtask", "build-guests", "--accelerated", "--check"],
        ["cargo", "xtask", "build-guests", "--accelerated"],
    )
    staged_accelerated = ROOT / "target/dist-accelerated/developer"
    require(staged_accelerated.is_dir(), "accelerated dist dir missing after build")
    durable_accelerated = WORK_ROOT / "accelerated"
    shutil.copytree(staged_accelerated, durable_accelerated)
    accelerated_staged_inventory = inventory(staged_accelerated)
    accelerated_inventory = inventory(durable_accelerated)
    require(
        accelerated_inventory == accelerated_staged_inventory,
        "accelerated snapshot copy inventory differs from the staged dist",
    )
    metadata_path = durable_accelerated / "build-metadata.toml"
    require(metadata_path.is_file(), "accelerated snapshot lacks build-metadata.toml")
    metadata_fields = {}
    for line in norm(metadata_path.read_text(encoding="utf-8")).splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            metadata_fields[key.strip()] = value.strip()
    require(
        metadata_fields
        == {
            "mode": '"accelerated"',
            "edition": '"developer"',
            "profile": '"release"',
            "perimeter_spatial_test_support": "false",
        },
        f"accelerated build metadata is not the expected mode identity: {metadata_fields}",
    )
    require(
        sorted(
            path.name
            for path in (durable_accelerated / "modules").iterdir()
            if path.is_dir()
        )
        == t41_stems,
        "accelerated snapshot module directory set differs from the t41 module set",
    )

    # Recheck the ordinary mode after the accelerated build.
    check_or_rebuild(
        "ordinary-after-accelerated",
        ["cargo", "xtask", "build-guests", "--check"],
        ["cargo", "xtask", "build-guests"],
    )
    # The durable ordinary snapshot must still agree with the in-tree artifacts
    # (in case the documented stale-rebuild path ran above).
    for stem in t41_stems:
        require(
            digest(ROOT / f"modules/core-modules/{stem}/{stem}.wasm")
            == digest(durable_ordinary / f"modules/{stem}/{stem}.wasm"),
            f"in-tree ordinary {stem} wasm diverged from the frozen snapshot",
        )

    # Source inputs must be unchanged after all builds.
    sources_after = source_inputs()
    require(
        sources_after == sources_before,
        "build inputs changed during snapshot builds",
    )
    write_json(
        EVIDENCE / "source-inputs-equality.json",
        {
            "before_after_equal": True,
            "matches_t41_source_inputs": True,
            "inputs_count": len(sources_before),
            "inputs_canonical_sha256": canonical_sources,
        },
    )

    # Per-guest mode-aware freshness identity files (actual files, read here).
    crate_by_stem = {}
    for stem in t41_stems:
        guest_manifest = ROOT / f"modules/core-modules/{stem}/wit-guest/Cargo.toml"
        require(guest_manifest.is_file(), f"missing guest manifest for {stem}")
        table = tomllib.loads(guest_manifest.read_text(encoding="utf-8-sig"))
        crate_by_stem[stem] = table["package"]["name"]
    require(
        len(set(crate_by_stem.values())) == len(crate_by_stem),
        "guest crate names are not unique",
    )
    descriptor = (
        f"cfg={policy['RESERVED_CFG']};target={policy['ALLOWED_WASM_TARGET']};"
        f"profile=guest-release;release={policy['ALLOWED_RELEASE']};"
        f"commit={policy['ALLOWED_COMMIT_HASH']};host={policy['ALLOWED_HOST']};"
        f"llvm={policy['ALLOWED_LLVM_VERSION']}"
    )
    ordinary_fingerprints = ROOT / "target/guest-fingerprints"
    accelerated_fingerprints = ROOT / "target/guest-fingerprints-accelerated"
    guest_records = {}
    for stem, crate in sorted(crate_by_stem.items()):
        ordinary_fingerprint = ordinary_fingerprints / f"{crate}.fingerprint"
        accelerated_fingerprint = accelerated_fingerprints / f"{crate}.fingerprint"
        accelerated_metadata = accelerated_fingerprints / f"{crate}.metadata"
        require(
            ordinary_fingerprint.is_file()
            and accelerated_fingerprint.is_file()
            and accelerated_metadata.is_file(),
            f"missing freshness identity files for guest {crate}",
        )
        metadata_text = norm(accelerated_metadata.read_text(encoding="utf-8"))
        lines = metadata_text.splitlines()
        require(
            lines[0] == "mode=accelerated", f"{crate} metadata mode is not accelerated"
        )
        require(lines[1] == f"policy={descriptor}", f"{crate} metadata policy differs")
        require(
            lines[2].startswith("policy-fingerprint=v2-"),
            f"{crate} metadata lacks a v2 policy fingerprint",
        )
        embedded_rustc = metadata_text.split("rustc-vV=", 1)[1]
        require(
            embedded_rustc.strip() == identity_text.strip(),
            f"{crate} metadata embeds a different rustc identity",
        )
        require(
            TEST_SUPPORT_FEATURE not in metadata_text,
            f"{crate} metadata carries a test-support marker",
        )
        guest_records[stem] = {
            "crate": crate,
            "ordinary_fingerprint_sha256": digest(ordinary_fingerprint),
            "accelerated_fingerprint_sha256": digest(accelerated_fingerprint),
            "accelerated_metadata_sha256": digest(accelerated_metadata),
        }

    # Module manifests must be the t41 frozen manifests; accelerated WASMs are
    # recorded as differences, never asserted equal or unequal.
    module_toml_matches = 0
    wasm_diff = []
    wasm_same = []
    for stem in t41_stems:
        ordinary_toml = durable_ordinary / f"modules/{stem}/{stem}.toml"
        accelerated_toml = durable_accelerated / f"modules/{stem}/{stem}.toml"
        expected_toml = t41_manifest["snapshot_sha256"][f"modules/{stem}/{stem}.toml"]
        require(
            digest(ordinary_toml) == expected_toml,
            f"ordinary {stem} manifest differs from the t41 frozen manifest",
        )
        require(
            digest(accelerated_toml) == expected_toml,
            f"accelerated {stem} manifest differs from the t41 frozen manifest",
        )
        module_toml_matches += 1
        ordinary_wasm = digest(ordinary_toml.with_suffix(".wasm"))
        accelerated_wasm = digest(accelerated_toml.with_suffix(".wasm"))
        if ordinary_wasm == accelerated_wasm:
            wasm_same.append(stem)
        else:
            wasm_diff.append(stem)

    # Supplementary artifact scan (record-only; the mode gate is the plan plus
    # the written metadata above).
    scanned = [durable_ordinary / "pnp_cli.exe", durable_accelerated / "pnp_cli.exe"]
    for stem in t41_stems:
        scanned.append(durable_ordinary / f"modules/{stem}/{stem}.wasm")
        scanned.append(durable_accelerated / f"modules/{stem}/{stem}.wasm")
    feature_hits = [
        str(path.relative_to(WORK_ROOT))
        for path in scanned
        if contains_bytes(path, TEST_SUPPORT_FEATURE.encode("ascii"))
    ]
    STATE["test_support_evidence"] = {
        "dist_plan_features": {
            "ordinary": parsed_ordinary_plan.get("features"),
            "accelerated": parsed_accelerated_plan.get("features"),
        },
        "accelerated_build_metadata": metadata_fields,
        "guest_metadata_test_support_marker_absent": True,
        "developer_edition_cargo_features_empty": True,
        "supplementary_artifact_scan_hits": feature_hits,
        "source_fact": (
            "xtask/src/dist.rs preflight_edition_for_mode rejects the "
            "TEST_SUPPORT_FEATURE in dist cargo features"
        ),
    }

    # Read-only module diagnosis for both snapshots: runner-default flags and
    # isolated flags, with SLICER_DEBUG_PATHS only for the diagnosis runs.
    discovery = json.loads(T41_DISCOVERY.read_text(encoding="utf-8-sig"))
    require(
        discovery.get("pass") is True and discovery.get("modules"),
        "t41 discovery evidence is not a passing nonempty report",
    )
    require(
        all(module["provenance"] == "external" for module in discovery["modules"]),
        "t41 discovery evidence contains a non-external module",
    )
    expected_ids = sorted(module["id"] for module in discovery["modules"])
    STATE["diagnose"] = {}
    for label, snapshot in (
        ("ordinary", durable_ordinary),
        ("accelerated", durable_accelerated),
    ):
        default_report = run_diagnose(
            f"{label}-runner-defaults", snapshot, [], expected_ids
        )
        isolated_report = run_diagnose(
            f"{label}-isolated",
            snapshot,
            ["--no-default-module-paths", "--no-integrated-modules"],
            expected_ids,
        )
        require(
            default_report["module_ids"] == isolated_report["module_ids"],
            f"{label} default and isolated diagnosis disagree on the module set",
        )
        STATE["diagnose"][label] = {
            "runner_default_flags": default_report,
            "isolated_flags": isolated_report,
        }
        for report in (default_report, isolated_report):
            shutil.copy2(
                LOG_DIR / Path(report["stdout_log"]).name,
                EVIDENCE / Path(report["stdout_log"]).name,
            )
            shutil.copy2(
                LOG_DIR / Path(report["stderr_log"]).name,
                EVIDENCE / Path(report["stderr_log"]).name,
            )

    # End: the read-only t41 identity gate again.
    after = run_cmd(
        "t41-freeze-verification-after",
        [sys.executable, T41 / "verify-freeze.py", T41_ATTEMPT],
    )
    after_summary = json.loads(after.stdout.decode("utf-8-sig"))
    require(after_summary.get("status") == "PASS", "t41 verifier after-run is not PASS")
    STATE["t41_gate_after"] = {
        "exit_code": 0,
        "summary_sha256": digest_bytes(after.stdout),
    }
    shutil.copy2(
        LOG_DIR / "t41-freeze-verification-after.stdout.log",
        EVIDENCE / "t41-freeze-verification-after.json",
    )

    # Frozen snapshot directories must contain no mutable logs.
    expected_ordinary_keys = {"pnp_cli.exe"} | {
        f"modules/{stem}/{stem}{extension}"
        for stem in t41_stems
        for extension in (".wasm", ".toml")
    }
    expected_accelerated_keys = expected_ordinary_keys | {"build-metadata.toml"}
    require(
        set(ordinary_inventory) == expected_ordinary_keys,
        "ordinary snapshot contains files outside the expected identity set",
    )
    require(
        set(accelerated_inventory) == expected_accelerated_keys,
        "accelerated snapshot contains files outside the expected identity set",
    )

    # The controlled-driver latch must be absent after the accelerated session.
    latch = ROOT / "target/accelerated-driver/latch"
    require(not latch.exists(), "a latched accelerated policy rejection is present")
    rustc_path_file = ROOT / "target/accelerated-driver/rustc-path"
    require(rustc_path_file.is_file(), "the saved real rustc path file is missing")
    saved_rustc = rustc_path_file.read_text(encoding="utf-8").strip()
    require(Path(saved_rustc).is_file(), "the saved real rustc path is not a file")

    target_side_effects = {
        "dist_developer_recreated": True,
        "dist_developer_matches_t41_frozen_snapshot": True,
        "dist_accelerated_created": (
            ROOT / "target/dist-accelerated/developer"
        ).is_dir()
        and not STATE["target_state_before"]["dist_accelerated_present"],
        "dist_host_accelerated_created": (
            ROOT / "target/dist-host-accelerated"
        ).is_dir()
        and not STATE["target_state_before"]["dist_host_accelerated_present"],
        "release_pnp_cli_sha256_after": digest(ROOT / "target/release/pnp_cli.exe"),
        "release_pnp_cli_matches_t41_snapshot_exe": digest(
            ROOT / "target/release/pnp_cli.exe"
        )
        == t41_manifest["snapshot_sha256"]["pnp_cli.exe"],
        "ordinary_guest_rebuild_performed": STATE["freshness_gates"]["ordinary"].get(
            "rebuild_performed", False
        ),
        "accelerated_guest_rebuild_performed": STATE["freshness_gates"][
            "accelerated"
        ].get("rebuild_performed", False),
        "driver_latch_absent": True,
        "saved_real_rustc": saved_rustc,
    }

    manifest = {
        "status": "frozen",
        "kind": "t38-campaign-snapshots",
        "created_utc": utc(),
        "purpose": STATE["purpose"],
        "durable_namespace": str(DURABLE),
        "durable_root": str(WORK_ROOT),
        "evidence_dir": str(EVIDENCE),
        "helper": {"path": str(HELPER), "sha256": digest(HELPER)},
        "t41_reference": {
            "attempt": str(T41_ATTEMPT),
            "manifest_sha256": t41_manifest_sha,
            "manifest_sidecar": (T41_ATTEMPT / "frozen-manifest.sha256")
            .read_text(encoding="utf-8")
            .strip(),
            "corpus_root": t41_manifest["corpus_root"],
            "snapshot_root": t41_manifest["snapshot_root"],
            "discovery_source": str(T41_DISCOVERY),
            "discovery_module_ids": expected_ids,
        },
        "compiler_identity": identity_text,
        "cargo_version": STATE["cargo_version"],
        "wasm_tools_version": STATE["wasm_tools_version"],
        "policy": STATE["policy"],
        "campaign_inputs": STATE["campaign_inputs"],
        "edition": {
            "name": "developer",
            "debug": False,
            "plan_ordinary": plan_ordinary_text,
            "plan_accelerated": plan_accelerated_text,
        },
        "commands": STATE["commands"],
        "source_identity": STATE["source_identity"],
        "ordinary_snapshot": STATE["ordinary_snapshot"],
        "accelerated_snapshot": {
            "root": str(durable_accelerated),
            "exe": str(durable_accelerated / "pnp_cli.exe"),
            "modules_dir": str(durable_accelerated / "modules"),
            "inventory_sha256": accelerated_inventory,
            "build_metadata": metadata_fields,
            "module_manifest_matches_t41": module_toml_matches,
        },
        "copy_identity": {
            "ordinary_matches_staged_dist": True,
            "accelerated_matches_staged_dist": True,
        },
        "previous_dist": previous_records,
        "guest_identity": {
            "stems": guest_records,
            "wasm_differ_from_ordinary": wasm_diff,
            "wasm_identical_to_ordinary": wasm_same,
            "embedded_rustc_and_policy_verified": True,
        },
        "test_support_evidence": STATE["test_support_evidence"],
        "freshness_gates": STATE["freshness_gates"],
        "diagnose": STATE["diagnose"],
        "t41_gate": {
            "before": STATE["t41_gate_before"],
            "after": STATE["t41_gate_after"],
        },
        "target_side_effects": target_side_effects,
        "mode_note": (
            "mode identity is read from the actual dist metadata and guest "
            "identity files, never inferred from a directory or binary name"
        ),
        "no_timing_or_tests": True,
        "environment_info": STATE["environment_info"],
        "gaps": STATE["gaps"],
    }

    manifest_path = WORK_ROOT / "snapshot-manifest.json"
    write_json(manifest_path, manifest)
    manifest_sha = digest(manifest_path)
    (WORK_ROOT / "snapshot-manifest.sha256").write_text(
        manifest_sha + "\n", encoding="utf-8"
    )
    shutil.copy2(manifest_path, EVIDENCE / "snapshot-manifest.json")
    (EVIDENCE / "snapshot-manifest.sha256").write_text(
        manifest_sha + "\n", encoding="utf-8"
    )

    (EVIDENCE / "FINDINGS.md").write_text(
        "\n".join(
            [
                "# t38 campaign snapshots (ordinary + accelerated)",
                "",
                "**Status: frozen.** Both mode-distinct developer snapshots were built once",
                "with the controlled entry points and copied into the durable namespace.",
                "No slice, test or timing command ran.",
                "",
                f"- durable root: `{WORK_ROOT}`",
                f"- manifest: `{manifest_path}`",
                f"- manifest sha256: `{manifest_sha}`",
                f"- ordinary exe: `{durable_ordinary / 'pnp_cli.exe'}`",
                f"- ordinary modules: `{durable_ordinary / 'modules'}`",
                f"- accelerated exe: `{durable_accelerated / 'pnp_cli.exe'}`",
                f"- accelerated modules: `{durable_accelerated / 'modules'}`",
                f"- t41 corpus root for `-CorpusRoot`: `{t41_manifest['corpus_root']}`",
                "",
                "All recorded gates exited 0: t41 read-only identity gate (before and",
                "after), exact rustc identity, source-input equality (before and after),",
                "both dist builds, both mode-specific freshness checks (ordinary also",
                "rechecked after the accelerated build), copy inventories, 24 per-guest",
                "accelerated metadata identities, and four read-only module-diagnosis",
                "probes (runner-default and isolated) showing 24 external modules with",
                "exactly one search root per snapshot.",
                "",
                "Accelerated WASMs differ from the ordinary WASMs for "
                f"{len(wasm_diff)} of {len(t41_stems)} modules; this is recorded,",
                "not asserted as a correctness signal. Frozen ordinary/accelerated",
                "subdirectories contain no mutable logs; logs live under the durable",
                "`logs/` directory and in this evidence directory.",
                "",
            ]
        ),
        encoding="utf-8",
    )

    STATE["status"] = "frozen"
    STATE["frozen_utc"] = utc()
    STATE["manifest"] = str(manifest_path)
    STATE["manifest_sha256"] = manifest_sha
    save_state()

    print(
        json.dumps(
            {
                "status": "SNAPSHOTS_READY",
                "ordinary": {
                    "exe": str(durable_ordinary / "pnp_cli.exe"),
                    "modules": str(durable_ordinary / "modules"),
                },
                "accelerated": {
                    "exe": str(durable_accelerated / "pnp_cli.exe"),
                    "modules": str(durable_accelerated / "modules"),
                },
                "manifest": str(manifest_path),
                "manifest_sha256": manifest_sha,
                "no_timing_or_tests": True,
            },
            indent=2,
        ),
        flush=True,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        STATE["status"] = "blocked"
        STATE["failure"] = str(error)
        try:
            save_state()
        except OSError:
            pass
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)
