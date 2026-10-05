"""Fail-closed, one-shot coordinator for the authorized t38 acceptance run."""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[6]
CAMPAIGN_DIR = Path(__file__).resolve().parent
ATTEMPT_DIR = CAMPAIGN_DIR / "attempt-1"
T38_DURABLE = (
    ROOT
    / ".local-artifacts/perimeter-reference-preparation/t38-campaign-20261001T024827Z"
)
SNAPSHOT_TRACKED = (
    ROOT
    / "docs/specs/perf-vs-orca/evidence/t38-adoption-resumed/snapshots/snapshot-manifest.json"
)
SNAPSHOT_DURABLE = T38_DURABLE / "snapshot-manifest.json"
T41_DIR = (
    ROOT / "docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/attempt-1"
)
T41_VERIFIER = (
    ROOT
    / "docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/verify-freeze.py"
)
T41_PYTHON = Path(r"C:\Users\agpen\.vfox\sdks\python\python.exe")
TARGET_ACCEPTANCE = ROOT / "target/perimeter-acceptance"
PREVIOUS_ACCEPTANCE = T38_DURABLE / "preserved-previous-acceptance"
RAW_ARCHIVE = T38_DURABLE / "campaign-raw-attempt1"
BENCH_TRACKED = ROOT / "resources/perimeter-acceptance/run_bench.ps1"
BENCH_AUTHORITY = ROOT / "docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1"

EXPECTED_SNAPSHOT_SHA256 = (
    "5c58b30145413df82ea9dc93a6b81bb778c69256550dc14d89ead7a61b654ec5"
)
EXPECTED_T41_MANIFEST_SHA256 = (
    "7b533240b7e602731eb6d11320c134c830fa00ec12b3d301b970dac935c334a3"
)
SAMPLES_PER_CELL = 4
THREADS = 12
WARMUPS_PER_CELL = 1


class StopBeforeCampaign(RuntimeError):
    pass


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def inventory(directory):
    directory = Path(directory)
    if not directory.is_dir():
        raise FileNotFoundError(f"missing directory: {directory}")
    return {
        path.relative_to(directory).as_posix(): sha256_file(path)
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def relative(path):
    return Path(path).resolve().relative_to(ROOT).as_posix()


def add_check(name, ok, details=None):
    entry = {
        "name": name,
        "exit_code": 0 if ok else 1,
        "status": "passed" if ok else "failed",
        "details": details or {},
    }
    state["checks"].append(entry)
    save_state()
    if not ok:
        raise StopBeforeCampaign(name)
    return entry


def save_state():
    state["updated_utc"] = utc_now()
    write_json(ATTEMPT_DIR / "results.json", state)


def run_command(name, argv, stdout_name, stderr_name):
    stdout_path = ATTEMPT_DIR / stdout_name
    stderr_path = ATTEMPT_DIR / stderr_name
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    entry = {
        "name": name,
        "argv": [str(part) for part in argv],
        "cwd": str(ROOT),
        "exit_code": None,
        "status": "started",
        "stdout_log": relative(stdout_path),
        "stderr_log": relative(stderr_path),
    }
    state["commands"].append(entry)
    save_state()
    try:
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            completed = subprocess.run(
                [str(part) for part in argv],
                cwd=ROOT,
                stdout=stdout,
                stderr=stderr,
                check=False,
            )
        entry["exit_code"] = completed.returncode
        entry["status"] = "completed"
    except OSError as error:
        entry["status"] = "launch-error"
        entry["error"] = f"{type(error).__name__}: {error}"
    save_state()
    return entry


def require_command_success(entry):
    if entry["exit_code"] != 0:
        raise StopBeforeCampaign(f"{entry['name']} exit was {entry['exit_code']}")


def t41_command(phase):
    return run_command(
        f"t41-freeze-verification-{phase}",
        [T41_PYTHON, T41_VERIFIER, T41_DIR],
        f"t41-freeze-{phase}.stdout.log",
        f"t41-freeze-{phase}.stderr.log",
    )


def verify_frozen_identities(phase):
    details = {}
    tracked_bytes = SNAPSHOT_TRACKED.read_bytes()
    durable_bytes = SNAPSHOT_DURABLE.read_bytes()
    tracked_digest = hashlib.sha256(tracked_bytes).hexdigest()
    durable_digest = hashlib.sha256(durable_bytes).hexdigest()
    tracked_sidecar = (
        SNAPSHOT_TRACKED.with_suffix(".sha256")
        .read_text(encoding="utf-8")
        .strip()
        .lower()
    )
    durable_sidecar = (
        SNAPSHOT_DURABLE.with_suffix(".sha256")
        .read_text(encoding="utf-8")
        .strip()
        .lower()
    )
    details["snapshot_manifest"] = {
        "tracked_sha256": tracked_digest,
        "durable_sha256": durable_digest,
        "tracked_sidecar": tracked_sidecar,
        "durable_sidecar": durable_sidecar,
        "expected_sha256": EXPECTED_SNAPSHOT_SHA256,
        "bytes_identical": tracked_bytes == durable_bytes,
        "all_digests_match": (
            tracked_digest
            == durable_digest
            == tracked_sidecar
            == durable_sidecar
            == EXPECTED_SNAPSHOT_SHA256
        ),
    }

    t41_manifest_path = T41_DIR / "frozen-manifest.json"
    t41_manifest_bytes = t41_manifest_path.read_bytes()
    t41_digest = hashlib.sha256(t41_manifest_bytes).hexdigest()
    t41_sidecar = (
        (T41_DIR / "frozen-manifest.sha256").read_text(encoding="utf-8").strip().lower()
    )
    t41_manifest = json.loads(t41_manifest_bytes.decode("utf-8-sig"))
    details["t41_manifest"] = {
        "sha256": t41_digest,
        "sidecar": t41_sidecar,
        "expected_sha256": EXPECTED_T41_MANIFEST_SHA256,
        "corpus_root": t41_manifest["corpus_root"],
        "snapshot_root": t41_manifest["snapshot_root"],
        "all_digests_match": t41_digest == t41_sidecar == EXPECTED_T41_MANIFEST_SHA256,
    }

    manifest = json.loads(tracked_bytes.decode("utf-8-sig"))
    details["corpus_root_matches_t41"] = (
        Path(manifest["t41_reference"]["corpus_root"]).resolve()
        == Path(t41_manifest["corpus_root"]).resolve()
    )
    details["campaign_inputs"] = {}
    for name, expected in manifest["campaign_inputs"].items():
        actual = sha256_file(ROOT / Path(name))
        details["campaign_inputs"][name] = {
            "expected": expected,
            "actual": actual,
            "matches": actual == expected,
        }

    details["snapshots"] = {}
    for key in ("ordinary_snapshot", "accelerated_snapshot"):
        snapshot = manifest[key]
        actual_inventory = inventory(snapshot["root"])
        expected_inventory = snapshot["inventory_sha256"]
        mismatches = sorted(
            name
            for name in set(actual_inventory) | set(expected_inventory)
            if actual_inventory.get(name) != expected_inventory.get(name)
        )
        details["snapshots"][key] = {
            "root": snapshot["root"],
            "file_count": len(actual_inventory),
            "inventory_matches": not mismatches,
            "mismatches": mismatches,
        }

    expected_modules = manifest["t41_reference"]["discovery_module_ids"]
    module_sets_match = True
    for mode in ("ordinary", "accelerated"):
        diagnose = manifest["diagnose"][mode]
        for discovery in ("runner_default_flags", "isolated_flags"):
            module_sets_match = (
                module_sets_match
                and diagnose[discovery]["module_ids"] == expected_modules
            )
    details["module_discovery_matches_frozen_24"] = module_sets_match
    details["snapshot_copy_identity"] = manifest["copy_identity"]

    all_inputs_match = all(
        item["matches"] for item in details["campaign_inputs"].values()
    )
    all_snapshots_match = all(
        item["inventory_matches"] for item in details["snapshots"].values()
    )
    ok = (
        details["snapshot_manifest"]["bytes_identical"]
        and details["snapshot_manifest"]["all_digests_match"]
        and details["t41_manifest"]["all_digests_match"]
        and details["corpus_root_matches_t41"]
        and all_inputs_match
        and all_snapshots_match
        and module_sets_match
        and all(manifest["copy_identity"].values())
    )
    add_check(f"frozen-identities-{phase}", ok, details)
    return manifest


def audit_environment():
    names = sorted(os.environ)
    overrides = [name for name in names if name.upper().startswith(("PNP_", "SLICER_"))]
    common_profile_metadata = {"ALLUSERSPROFILE", "USERPROFILE", "WT_PROFILE_ID"}
    suspicious_name_pattern = re.compile(
        r"(?i)(?:^|_)(?:PROFILE|PROFILING|FUEL|DEBUG|INSTRUMENT|CAPTURE|BACKTRACE)(?:_|$)"
    )
    suspicious_names = [
        name
        for name in names
        if name.upper() not in common_profile_metadata
        and suspicious_name_pattern.search(name)
    ]
    value_names = []
    value_check_names = {
        "RUST_LOG",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "LLVM_PROFILE_FILE",
        "CARGO_PROFILE_RELEASE_DEBUG",
        "CARGO_PROFILE_DEV_DEBUG",
    }
    value_pattern = re.compile(
        r"(?i)(?:--instrument-stderr|debuginfo|profile-(?:generate|use)|instrument-coverage|"
        r"\b(?:debug|trace|profile|profil(?:e|ing)|fuel|instrument(?:ed)?|capture)\b)"
    )
    for name, value in os.environ.items():
        if name.upper() in value_check_names and value_pattern.search(value):
            value_names.append(name)
    details = {
        "pnp_or_slicer_override_names": overrides,
        "profiling_fuel_debug_environment_names": sorted(
            set(suspicious_names + value_names)
        ),
        "values_recorded": False,
    }
    add_check(
        "environment-overrides",
        not overrides and not details["profiling_fuel_debug_environment_names"],
        details,
    )


def preserve_previous_target():
    runs_dir = TARGET_ACCEPTANCE / "runs"
    existing_run_entries = (
        sorted(path.relative_to(runs_dir).as_posix() for path in runs_dir.rglob("*"))
        if runs_dir.exists()
        else []
    )
    if existing_run_entries:
        add_check(
            "runner-target-has-no-prior-runs",
            False,
            {"runs_directory": str(runs_dir), "existing_entries": existing_run_entries},
        )
    add_check(
        "runner-target-has-no-prior-runs",
        True,
        {"runs_directory": str(runs_dir), "existing_entries": existing_run_entries},
    )

    before = inventory(TARGET_ACCEPTANCE) if TARGET_ACCEPTANCE.is_dir() else {}
    if PREVIOUS_ACCEPTANCE.exists():
        add_check(
            "preserve-prior-acceptance",
            False,
            {"destination_already_exists": str(PREVIOUS_ACCEPTANCE)},
        )
    if TARGET_ACCEPTANCE.is_dir():
        shutil.copytree(TARGET_ACCEPTANCE, PREVIOUS_ACCEPTANCE)
        copied = inventory(PREVIOUS_ACCEPTANCE)
        ok = copied == before
    else:
        PREVIOUS_ACCEPTANCE.mkdir(parents=True)
        copied = {}
        ok = not before
    add_check(
        "preserve-prior-acceptance",
        ok,
        {
            "source": str(TARGET_ACCEPTANCE),
            "destination": str(PREVIOUS_ACCEPTANCE),
            "source_inventory": before,
            "copied_inventory": copied,
            "original_retained": TARGET_ACCEPTANCE.is_dir(),
        },
    )


def check_benchmark_copy():
    tracked = BENCH_TRACKED.read_bytes()
    authority = BENCH_AUTHORITY.read_bytes()
    details = {
        "tracked_path": str(BENCH_TRACKED),
        "authority_path": str(BENCH_AUTHORITY),
        "tracked_sha256": hashlib.sha256(tracked).hexdigest(),
        "authority_sha256": hashlib.sha256(authority).hexdigest(),
        "bytes_identical": tracked == authority,
    }
    add_check("benchmark-copy-byte-identity", details["bytes_identical"], details)


def verify_dry_run():
    summary_path = TARGET_ACCEPTANCE / "summary.json"
    if not summary_path.is_file():
        add_check("dry-run-protocol", False, {"summary_missing": str(summary_path)})
    summary_bytes = summary_path.read_bytes()
    (ATTEMPT_DIR / "dry-run.synthetic.summary.json").write_bytes(summary_bytes)
    summary = json.loads(summary_bytes.decode("utf-8-sig"))
    cells = summary.get("cells", [])
    all_paths_synthetic = all(
        str(row.get("output_path", "")).startswith("synthetic://")
        for cell in cells
        for variant in ("baseline", "candidate")
        for row in cell.get(variant, {}).get("rows", [])
    )
    overlap_cell = bool(cells) and cells[0].get("decision") == "inconclusive"
    exactness_drop = any(
        cell.get("decision") == "DROP" and cell.get("exactness_passed") is False
        for cell in cells
    )
    runs_entries = (
        sorted(
            path.relative_to(TARGET_ACCEPTANCE).as_posix()
            for path in (TARGET_ACCEPTANCE / "runs").rglob("*")
        )
        if (TARGET_ACCEPTANCE / "runs").exists()
        else []
    )
    details = {
        "summary_path": relative(ATTEMPT_DIR / "dry-run.synthetic.summary.json"),
        "status": summary.get("status"),
        "automatic_commit": summary.get("automatic_commit"),
        "threads": summary.get("threads"),
        "samples_per_cell": summary.get("samples_per_cell"),
        "warmup_per_cell": summary.get("warmup_per_cell"),
        "cell_count": len(cells),
        "all_output_paths_synthetic": all_paths_synthetic,
        "overlap_rejection_present": overlap_cell,
        "exactness_drop_present": exactness_drop,
        "no_runner_run_artifacts": not runs_entries,
        "runner_run_entries": runs_entries,
        "timings_are_synthetic_only": True,
    }
    ok = (
        summary.get("status") == "DROP"
        and summary.get("automatic_commit") is False
        and summary.get("threads") == THREADS
        and summary.get("samples_per_cell") == SAMPLES_PER_CELL
        and summary.get("warmup_per_cell") == WARMUPS_PER_CELL
        and len(cells) == 6
        and all_paths_synthetic
        and overlap_cell
        and exactness_drop
        and not runs_entries
    )
    add_check("dry-run-protocol", ok, details)


def copy_runner_artifacts_and_hashes():
    summary_source = TARGET_ACCEPTANCE / "summary.json"
    if summary_source.is_file():
        shutil.copy2(summary_source, ATTEMPT_DIR / "campaign-summary.json")
        state["campaign_summary_captured"] = True
    else:
        state["campaign_summary_captured"] = False
    save_state()

    if not RAW_ARCHIVE.exists():
        if TARGET_ACCEPTANCE.is_dir():
            shutil.copytree(TARGET_ACCEPTANCE, RAW_ARCHIVE)
            state["raw_archive_status"] = "copied"
        else:
            state["raw_archive_status"] = "missing-target-directory"
    else:
        state["raw_archive_status"] = "destination-already-exists"
    save_state()

    tracked_payload_root = ATTEMPT_DIR / "runner-artifacts/target/perimeter-acceptance"
    if TARGET_ACCEPTANCE.is_dir():
        for path in sorted(TARGET_ACCEPTANCE.rglob("*")):
            if not path.is_file() or path.suffix.lower() == ".gcode":
                continue
            destination = tracked_payload_root / path.relative_to(TARGET_ACCEPTANCE)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
    state["tracked_campaign_payload"] = relative(tracked_payload_root)
    save_state()

    if RAW_ARCHIVE.is_dir():
        raw_inventory = inventory(RAW_ARCHIVE)
        write_json(
            RAW_ARCHIVE / "hash-inventory.json",
            {
                "algorithm": "sha256",
                "scope": "all files copied from target/perimeter-acceptance before adding this inventory",
                "files": raw_inventory,
            },
        )
        state["raw_archive_inventory_sha256"] = sha256_file(
            RAW_ARCHIVE / "hash-inventory.json"
        )
        state["raw_archive_file_count"] = len(raw_inventory)
    else:
        state["raw_archive_inventory_sha256"] = None
        state["raw_archive_file_count"] = 0
    save_state()


def compute_stats(values):
    values = [float(value) for value in values]
    if not values:
        return None
    ordered = sorted(values)
    middle = len(ordered) // 2
    median = (
        ordered[middle]
        if len(ordered) % 2
        else (ordered[middle - 1] + ordered[middle]) / 2.0
    )
    return {"min": min(values), "max": max(values), "median": median, "samples": values}


def summarize_cells(summary):
    result = []
    for cell in summary.get("cells", []):
        decision = cell.get("decision", "unknown")
        exactness = (
            "not-run"
            if decision == "not-run"
            else bool(cell.get("exactness_passed", False))
        )
        variants = {}
        for variant in ("baseline", "candidate"):
            rows = cell.get(variant, {}).get("rows", []) or []
            cpu = compute_stats([row["cpu_seconds"] for row in rows])
            wall = compute_stats([row["wall_seconds"] for row in rows])
            variants[variant] = {
                "retained_rows": len(rows),
                "cpu_seconds": cpu,
                "wall_seconds": wall,
                "status_values": [row.get("status", "") for row in rows],
                "generator_markers": [row.get("generator_marker", "") for row in rows],
                "degraded_values": [row.get("degraded", 0) for row in rows],
                "nonfatal_error_values": [
                    row.get("nonfatal_errors", 0) for row in rows
                ],
                "fatal_error_values": [row.get("fatal_errors", 0) for row in rows],
            }
        cpu_ratio = None
        wall_ratio = None
        if (
            variants["baseline"]["cpu_seconds"]
            and variants["baseline"]["cpu_seconds"]["median"] != 0
            and variants["candidate"]["cpu_seconds"]
        ):
            cpu_ratio = (
                variants["candidate"]["cpu_seconds"]["median"]
                / variants["baseline"]["cpu_seconds"]["median"]
            )
        if (
            variants["baseline"]["wall_seconds"]
            and variants["baseline"]["wall_seconds"]["median"] != 0
            and variants["candidate"]["wall_seconds"]
        ):
            wall_ratio = (
                variants["candidate"]["wall_seconds"]["median"]
                / variants["baseline"]["wall_seconds"]["median"]
            )
        result.append(
            {
                "workload": cell.get("workload"),
                "generator": cell.get("generator"),
                "decision": decision,
                "exactness": exactness,
                "exactness_deltas": cell.get("exactness_deltas"),
                "config_provenance": cell.get("config_provenance"),
                "expected_generator": cell.get("expected_generator"),
                "generator_marker": cell.get("generator_marker"),
                "generator_marker_match": cell.get("generator_marker_match"),
                "status_baseline": cell.get("status_baseline", ""),
                "status_candidate": cell.get("status_candidate", ""),
                "degraded_baseline": cell.get("degraded_baseline", 0),
                "degraded_candidate": cell.get("degraded_candidate", 0),
                "nonfatal_errors_baseline": cell.get("nonfatal_errors_baseline", 0),
                "nonfatal_errors_candidate": cell.get("nonfatal_errors_candidate", 0),
                "fatal_errors_baseline": cell.get("fatal_errors_baseline", 0),
                "fatal_errors_candidate": cell.get("fatal_errors_candidate", 0),
                "cpu_separated": cell.get("cpu_separated"),
                "wall_separated": cell.get("wall_separated"),
                "cpu_ratio_candidate_over_baseline": cpu_ratio,
                "wall_ratio_candidate_over_baseline": wall_ratio,
                "baseline": variants["baseline"],
                "candidate": variants["candidate"],
            }
        )
    return result


def format_stat(stat):
    if stat is None:
        return "not run"
    return f"{stat['min']:.4f}–{stat['max']:.4f}; median {stat['median']:.4f}"


def write_findings():
    summary = None
    summary_path = ATTEMPT_DIR / "campaign-summary.json"
    if summary_path.is_file():
        try:
            summary = read_json(summary_path)
        except (json.JSONDecodeError, UnicodeDecodeError):
            state["campaign_summary_parse_error"] = True
    corpus_root = Path(manifest["t41_reference"]["corpus_root"]).resolve()
    provenance = summary.get("config_provenance", []) if summary else []
    summary_is_real = (
        bool(summary)
        and len(provenance) == 6
        and all(
            not str(item.get("path", "")).startswith("synthetic://")
            and Path(item.get("path", "")).resolve().is_relative_to(corpus_root)
            and bool(item.get("config_sha256"))
            for item in provenance
        )
    )
    cells = summarize_cells(summary) if summary_is_real else []
    state["cells"] = cells
    state["runner_summary_source"] = (
        "real-campaign" if summary_is_real else "missing-or-noncampaign"
    )
    state["runner_status"] = summary.get("status") if summary_is_real else None
    state["automatic_commit"] = summary.get("automatic_commit") if summary else None
    state["runner_protocol"] = {
        "summary_present": summary is not None,
        "summary_is_real_not_synthetic": summary_is_real,
        "threads": summary.get("threads") if summary else None,
        "samples_per_cell": summary.get("samples_per_cell") if summary else None,
        "warmup_per_cell": summary.get("warmup_per_cell") if summary else None,
        "cell_count": len(summary.get("cells", [])) if summary else 0,
        "config_provenance_count": len(summary.get("config_provenance", []))
        if summary
        else 0,
    }
    if summary:
        expected_protocol = (
            state["runner_protocol"]["summary_is_real_not_synthetic"]
            and summary.get("threads") == THREADS
            and summary.get("samples_per_cell") == SAMPLES_PER_CELL
            and summary.get("warmup_per_cell") == WARMUPS_PER_CELL
            and len(summary.get("cells", [])) == 6
            and len(summary.get("config_provenance", [])) == 6
            and all(
                str(item.get("path", "")).startswith(
                    str(Path(manifest["t41_reference"]["corpus_root"]).resolve())
                )
                and bool(item.get("config_sha256"))
                for item in summary.get("config_provenance", [])
            )
            and summary.get("automatic_commit") is False
        )
    else:
        expected_protocol = False
    state["runner_protocol"]["passed"] = expected_protocol

    command = next(
        (
            item
            for item in state["commands"]
            if item["name"] == "ordinary-vs-accelerated-campaign"
        ),
        None,
    )
    campaign_exit = command.get("exit_code") if command else None
    state["campaign_exit_code"] = campaign_exit
    actual_decisions = [
        cell["decision"] for cell in cells if cell["decision"] != "not-run"
    ]
    if not summary or not expected_protocol:
        runner_outcome = "BLOCKED"
        reason = "No valid real campaign summary was produced."
    elif "DROP" in actual_decisions:
        runner_outcome = "DROP"
        reason = "At least one executed cell failed the existing acceptance gate."
    elif "inconclusive" in actual_decisions:
        runner_outcome = "INCONCLUSIVE"
        reason = "An executed cell had overlapping or touching measured ranges."
    elif campaign_exit != 0:
        runner_outcome = "BLOCKED"
        reason = "The campaign command did not exit successfully."
    elif any(cell["decision"] == "not-run" for cell in cells):
        runner_outcome = "BLOCKED"
        reason = "The campaign stopped with cells not run and no executed DROP/inconclusive decision."
    elif len(cells) == 6 and all(cell["decision"] == "KEEP" for cell in cells):
        runner_outcome = "KEEP"
        reason = "Every cell passed exactness, status checks, and strict favorable CPU and wall separation."
    else:
        runner_outcome = "BLOCKED"
        reason = "The campaign result did not satisfy a complete recognized outcome."

    first_stopping_cell = next(
        (cell for cell in cells if cell["decision"] not in ("KEEP", "not-run")),
        None,
    )
    if first_stopping_cell:
        if first_stopping_cell["decision"] == "inconclusive":
            cell_cause = "CPU or wall measured ranges overlap or touch"
        elif first_stopping_cell["exactness"] is False:
            cell_cause = (
                "baseline/candidate exactness failed against the frozen reference"
            )
        elif not first_stopping_cell["generator_marker_match"]:
            cell_cause = "generator marker mismatch"
        elif (
            first_stopping_cell["status_baseline"]
            != first_stopping_cell["status_candidate"]
        ):
            cell_cause = "completion status mismatch"
        elif (
            first_stopping_cell["degraded_baseline"]
            != first_stopping_cell["degraded_candidate"]
        ):
            cell_cause = "degraded status mismatch"
        elif (
            first_stopping_cell["nonfatal_errors_baseline"]
            != first_stopping_cell["nonfatal_errors_candidate"]
        ):
            cell_cause = "non-fatal error count mismatch"
        elif (
            first_stopping_cell["fatal_errors_baseline"]
            or first_stopping_cell["fatal_errors_candidate"]
        ):
            cell_cause = "fatal errors present"
        else:
            cell_cause = (
                "measured CPU/wall ranges did not both strictly favor the candidate"
            )
        state["first_stopping_cell"] = {
            "workload": first_stopping_cell["workload"],
            "generator": first_stopping_cell["generator"],
            "decision": first_stopping_cell["decision"],
            "cause": cell_cause,
        }

    post_ok = state.get("post_campaign_integrity_passed") is True
    archive_ok = (
        state.get("raw_archive_status") == "copied"
        and state.get("raw_archive_inventory_sha256") is not None
    )
    evidence_ok = state.get("tracked_payload_copy_status") == "copied"
    if runner_outcome == "KEEP" and (not post_ok or not archive_ok or not evidence_ok):
        runner_outcome = "BLOCKED"
        reason = "Runner KEEP was not accepted because post-run integrity or evidence preservation failed."
    elif runner_outcome in ("DROP", "INCONCLUSIVE") and (
        not post_ok or not archive_ok or not evidence_ok
    ):
        state["runner_outcome_before_integrity_override"] = runner_outcome
        runner_outcome = "BLOCKED"
        reason = "Runner decision was recorded, but post-run integrity or evidence preservation failed."

    state["outcome"] = runner_outcome
    state["outcome_reason"] = reason
    state["retained_measured_rows"] = sum(
        cell["baseline"]["retained_rows"] + cell["candidate"]["retained_rows"]
        for cell in cells
    )
    state["retained_measured_sample_pairs"] = sum(
        min(cell["baseline"]["retained_rows"], cell["candidate"]["retained_rows"])
        for cell in cells
    )
    if (
        state["retained_measured_rows"] == 0
        and first_stopping_cell
        and first_stopping_cell["exactness"] is False
    ):
        state["timing_status"] = "not-run: exactness failed before measured samples"
    elif state["retained_measured_rows"] == 0:
        state["timing_status"] = "no retained measured rows"
    else:
        state["timing_status"] = "retained measured rows summarized"
    save_state()

    lines = [
        "# Issue 38 campaign — attempt 1 findings",
        "",
        f"- **FACT outcome:** {runner_outcome}",
        f"- **Runner status / actual campaign exit:** {state['runner_status']} / {campaign_exit}",
        f"- **Reason:** {reason}",
        f"- **First stopping cell / cause:** {state.get('first_stopping_cell', 'none recorded')}",
        f"- **Retained measured rows / paired sample count:** {state['retained_measured_rows']} / {state['retained_measured_sample_pairs']}",
        f"- **Timing status:** {state['timing_status']}.",
        f"- **Post-campaign t41 and frozen-input identity:** {'PASS' if post_ok else 'FAIL or unavailable'}",
        f"- **Raw archive:** `{state.get('raw_archive_path', 'not created')}` ({state.get('raw_archive_status', 'not created')})",
        "- Synthetic status-roundtrip and dry-run values are controls only and are excluded from all timing results.",
        "- CPU and wall statistics below are computed only from retained measured rows in the real campaign summary; exactness and warmup rows are excluded.",
        "",
        "| Cell | Decision | Exactness | Retained B/C | Status B/C | Marker B/C | Degraded B/C | Nonfatal B/C | Fatal B/C | CPU B/C (s: range; median) | Wall B/C (s: range; median) | CPU / wall ratio | Exactness deltas |",
        "|---|---|---|---:|---|---|---|---|---|---|---|---|---|",
    ]
    for cell in cells:
        exactness = cell["exactness"]
        exactness_text = (
            "not-run" if exactness == "not-run" else ("pass" if exactness else "fail")
        )
        markers = cell.get("generator_marker") or {}
        marker_text = f"{markers.get('baseline', '')}/{markers.get('candidate', '')}"
        cpu_text = f"{format_stat(cell['baseline']['cpu_seconds'])} / {format_stat(cell['candidate']['cpu_seconds'])}"
        wall_text = f"{format_stat(cell['baseline']['wall_seconds'])} / {format_stat(cell['candidate']['wall_seconds'])}"
        ratios = f"{cell['cpu_ratio_candidate_over_baseline']!s} / {cell['wall_ratio_candidate_over_baseline']!s}"
        deltas = json.dumps(cell.get("exactness_deltas"), separators=(",", ":"))
        lines.append(
            f"| {cell['workload']} / {cell['generator']} | {cell['decision']} | {exactness_text} | "
            f"{cell['baseline']['retained_rows']}/{cell['candidate']['retained_rows']} | "
            f"{cell['status_baseline']}/{cell['status_candidate']} | {marker_text} | "
            f"{cell['degraded_baseline']}/{cell['degraded_candidate']} | "
            f"{cell['nonfatal_errors_baseline']}/{cell['nonfatal_errors_candidate']} | "
            f"{cell['fatal_errors_baseline']}/{cell['fatal_errors_candidate']} | "
            f"{cpu_text} | {wall_text} | {ratios} | `{deltas}` |"
        )
    if not cells:
        lines.append(
            "| — | no valid summary | not-run | 0/0 | — | — | — | — | — | timing not run | timing not run | — | — |"
        )
    lines.extend(
        [
            "",
            "Machine-readable cell measurements and commands: [`results.json`](results.json).",
            "Synthetic dry-run summary (not acceptance evidence): [`dry-run.synthetic.summary.json`](dry-run.synthetic.summary.json).",
            "Campaign runner summary: [`campaign-summary.json`](campaign-summary.json) when present.",
            "Tracked raw logs/CSV/process diagnostics exclude licensed G-code; complete raw output is retained only in the durable archive.",
            "",
        ]
    )
    (ATTEMPT_DIR / "FINDINGS.md").write_text("\n".join(lines), encoding="utf-8")


def finalize_hash_inventory():
    raw_files = inventory(RAW_ARCHIVE) if RAW_ARCHIVE.is_dir() else {}
    raw_files.pop("hash-inventory.json", None)
    tracked_files = inventory(ATTEMPT_DIR)
    tracked_files.pop("hash-inventory.json", None)
    write_json(
        ATTEMPT_DIR / "hash-inventory.json",
        {
            "algorithm": "sha256",
            "tracked_evidence_files_excluding_this_inventory": tracked_files,
            "durable_raw_archive": str(RAW_ARCHIVE),
            "durable_raw_archive_files_excluding_its_inventory": raw_files,
            "durable_raw_archive_inventory_sha256": state.get(
                "raw_archive_inventory_sha256"
            ),
        },
    )


def make_preflight_blocked(reason):
    state["outcome"] = "BLOCKED"
    state["outcome_reason"] = str(reason)
    state["campaign_started"] = False
    state["retained_measured_rows"] = 0
    state["retained_measured_sample_pairs"] = 0
    save_state()
    (ATTEMPT_DIR / "FINDINGS.md").write_text(
        "# Issue 38 campaign — attempt 1 findings\n\n"
        f"- **FACT outcome:** BLOCKED\n- **Reason:** {reason}\n"
        "- Campaign timing did not run; no measured rows were retained.\n"
        "- No retry is authorized by this attempt.\n",
        encoding="utf-8",
    )
    finalize_hash_inventory()


if ATTEMPT_DIR.exists():
    raise SystemExit(
        f"one-shot guard: attempt directory already exists; refusing to rerun: {ATTEMPT_DIR}"
    )

ATTEMPT_DIR.mkdir(parents=True)
state = {
    "packet": "issue38-accelerated-adoption-acceptance-campaign",
    "attempt": 1,
    "started_utc": utc_now(),
    "helper": relative(Path(__file__)),
    "helper_argv": [sys.executable, str(Path(__file__).resolve())],
    "commands": [],
    "checks": [],
    "outcome": "BLOCKED",
    "campaign_started": False,
    "raw_archive_path": str(RAW_ARCHIVE),
}
save_state()

try:
    before = t41_command("before")
    require_command_success(before)
    manifest = verify_frozen_identities("before")
    audit_environment()
    if RAW_ARCHIVE.exists():
        add_check(
            "durable-destinations-unused",
            False,
            {"raw_archive_exists": str(RAW_ARCHIVE)},
        )
    if PREVIOUS_ACCEPTANCE.exists():
        add_check(
            "durable-destinations-unused",
            False,
            {"preserved_acceptance_exists": str(PREVIOUS_ACCEPTANCE)},
        )
    add_check(
        "durable-destinations-unused",
        True,
        {
            "raw_archive": str(RAW_ARCHIVE),
            "preserved_acceptance": str(PREVIOUS_ACCEPTANCE),
        },
    )
    preserve_previous_target()
    check_benchmark_copy()

    status = run_command(
        "status-roundtrip-control",
        [
            "pwsh",
            "-NoProfile",
            "-File",
            "resources/perimeter-acceptance/test-status-roundtrip.ps1",
        ],
        "status-roundtrip.stdout.log",
        "status-roundtrip.stderr.log",
    )
    require_command_success(status)

    dry_run = run_command(
        "runner-dry-run-control",
        [
            "pwsh",
            "-NoProfile",
            "-File",
            "resources/perimeter-acceptance/run-acceptance.ps1",
            "-DryRun",
        ],
        "dry-run.stdout.log",
        "dry-run.stderr.log",
    )
    require_command_success(dry_run)
    verify_dry_run()

except StopBeforeCampaign as error:
    make_preflight_blocked(error)
    print(f"FACT BLOCKED: {error}")
    print(f"Evidence: {ATTEMPT_DIR}")
    raise SystemExit(2)
except Exception as error:
    make_preflight_blocked(
        f"pre-campaign helper error: {type(error).__name__}: {error}"
    )
    print(f"FACT BLOCKED: pre-campaign helper error: {type(error).__name__}: {error}")
    print(f"Evidence: {ATTEMPT_DIR}")
    raise SystemExit(2)


corpus_root = manifest["t41_reference"]["corpus_root"]
ordinary = manifest["ordinary_snapshot"]
accelerated = manifest["accelerated_snapshot"]
campaign_argv = [
    "pwsh",
    "-NoProfile",
    "-File",
    "resources/perimeter-acceptance/run-acceptance.ps1",
    "-Campaign",
    "-CorpusRoot",
    corpus_root,
    "-Threads",
    "12",
    "-BaselineExePath",
    ordinary["exe"],
    "-CandidateExePath",
    accelerated["exe"],
    "-BaselineModuleDir",
    ordinary["modules_dir"],
    "-CandidateModuleDir",
    accelerated["modules_dir"],
]
state["campaign_started"] = True
state["campaign_started_utc"] = utc_now()
save_state()
campaign_command = run_command(
    "ordinary-vs-accelerated-campaign",
    campaign_argv,
    "campaign.stdout.log",
    "campaign.stderr.log",
)

try:
    copy_runner_artifacts_and_hashes()
    state["tracked_campaign_payload_copy_status"] = "copied"
except Exception as error:
    state["tracked_campaign_payload_copy_status"] = (
        f"copy-error: {type(error).__name__}: {error}"
    )
    state["evidence_archive_error"] = state["tracked_campaign_payload_copy_status"]
    save_state()

try:
    after = t41_command("after")
    state["t41_after_exit_code"] = after["exit_code"]
except Exception as error:
    state["t41_after_exit_code"] = None
    state["t41_after_error"] = f"{type(error).__name__}: {error}"
    save_state()

try:
    verify_frozen_identities("after")
    post_identity_ok = state["checks"][-1]["status"] == "passed"
except Exception as error:
    post_identity_ok = False
    state["post_identity_error"] = f"{type(error).__name__}: {error}"
    save_state()

t41_after_ok = state.get("t41_after_exit_code") == 0
state["post_campaign_integrity_passed"] = bool(post_identity_ok and t41_after_ok)
state["campaign_exit_code"] = campaign_command["exit_code"]
state["completed_utc"] = utc_now()
save_state()

try:
    write_findings()
except Exception as error:
    state["outcome"] = "BLOCKED"
    state["outcome_reason"] = (
        f"result synthesis failed: {type(error).__name__}: {error}"
    )
    state["result_synthesis_error"] = state["outcome_reason"]
    save_state()
    (ATTEMPT_DIR / "FINDINGS.md").write_text(
        "# Issue 38 campaign — attempt 1 findings\n\n"
        "- **FACT outcome:** BLOCKED\n"
        f"- **Reason:** {state['outcome_reason']}\n"
        "- Inspect campaign-summary.json, command logs, and durable raw archive; no retry was attempted.\n",
        encoding="utf-8",
    )
finalize_hash_inventory()

print(f"FACT {state['outcome']}: {state['outcome_reason']}")
print(
    f"Campaign command exit: {state.get('campaign_exit_code')}; runner status: {state.get('runner_status')}"
)
print(f"Retained measured rows: {state.get('retained_measured_rows', 0)}")
print(f"Evidence: {ATTEMPT_DIR}")
print(f"Raw archive: {RAW_ARCHIVE}")
