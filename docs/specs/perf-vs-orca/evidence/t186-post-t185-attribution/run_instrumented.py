#!/usr/bin/env python3
"""t186 post-t185 full-matrix attribution -- instrumented passes (3 per cell).

Attribution only: `--instrument-stderr` inflates wall, so nothing here is a
wall claim. Every pass is checked for exit_code == 0, degraded == False,
fatal/non_fatal == 0, code-1200 diagnostics == 0, and byte-identical G-code
SHA-256 against t182's active reference set.

Usage:
  python run_instrumented.py                 # 8 cells x 3 passes
  python run_instrumented.py --passes 1      # smoke
  python run_instrumented.py --cells benchy-classic-on,benchy-arachne-on
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def module_tree_identity(mod_dir: Path) -> dict:
    wasms = sorted(mod_dir.rglob("*.wasm"), key=lambda p: p.relative_to(mod_dir).as_posix())
    lines = []
    files = {}
    for w in wasms:
        rel = w.relative_to(mod_dir).as_posix()
        digest = sha256_file(w)
        files[rel] = digest
        lines.append(f"{rel} {digest}\n")
    tree_sha = hashlib.sha256("".join(lines).encode("utf-8")).hexdigest()
    return {
        "module_dir": mod_dir.relative_to(REPO).as_posix(),
        "module_count": len(wasms),
        "wasm_tree_sha256": tree_sha,
        "wasm_files": files,
    }


def git_rev(ref: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(REPO), "rev-parse", ref], text=True
    ).strip()


def last_code_commit() -> str:
    return subprocess.check_output(
        [
            "git",
            "-C",
            str(REPO),
            "log",
            "-n",
            "1",
            "--format=%H",
            "--",
            "crates",
            "modules",
            "vendor",
            "xtask",
            "Cargo.toml",
            "Cargo.lock",
        ],
        text=True,
    ).strip()


def inspect_jsonl(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    with raw.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if (
                '"code":1200' in line
                or '"code": 1200' in line
                or "exact-Z occupancy" in line
            ):
                code_1200 += 1
            s = line.strip()
            if not s.startswith("{"):
                continue
            try:
                obj = json.loads(s)
            except json.JSONDecodeError:
                continue
            if obj.get("event") == "slice_complete":
                degraded = obj.get("degraded")
                fatal = obj.get("fatal_error_count")
                non_fatal = obj.get("non_fatal_error_count")
                slice_complete_ms = obj.get("elapsed_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
    }


def reference_data() -> tuple[dict[str, str], dict[str, int]]:
    doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    return doc["active_set"], doc["bytes"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--passes", type=int, default=3)
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t186-instrumented"))
    a = ap.parse_args()

    refs, ref_bytes = reference_data()
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    identities = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "git_head": git_rev("HEAD"),
        "last_code_commit": last_code_commit(),
        "ordinary_guests_check_exit": 0,
        "rayon_num_threads": 12,
        "exe_path": EXE.relative_to(REPO).as_posix(),
        "exe_sha256": sha256_file(EXE),
        "modules": module_tree_identity(MODULES),
        "reference_artifact": REFERENCE.relative_to(REPO).as_posix(),
    }

    failures = 0
    rows = []
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        for p in range(1, a.passes + 1):
            gcode = out_dir / f"{cell}.pass{p}.gcode"
            raw = out_dir / f"{cell}.pass{p}.jsonl"
            args = [
                str(EXE),
                "slice",
                "--model",
                str(FIXTURES[fixture]),
                "--output",
                str(gcode),
                "--config",
                str(cfg),
                "--module-dir",
                str(MODULES),
                "--no-default-module-paths",
                "--instrument-stderr",
            ]
            env = dict(os.environ, RAYON_NUM_THREADS="12")
            t0 = time.perf_counter()
            with raw.open("wb") as errf:
                rc = subprocess.run(
                    args, stdout=subprocess.DEVNULL, stderr=errf, env=env
                ).returncode
            wall = time.perf_counter() - t0
            got = sha256_file(gcode) if gcode.exists() else ""
            got_bytes = gcode.stat().st_size if gcode.exists() else 0
            want = refs.get(cell)
            want_bytes = ref_bytes.get(cell)
            ok_ref = (want is not None and got == want) and (
                want_bytes is None or got_bytes == want_bytes
            )
            ev = inspect_jsonl(raw)
            ok_clean = (
                rc == 0
                and ok_ref
                and ev["degraded"] is False
                and ev["fatal_error_count"] == 0
                and ev["non_fatal_error_count"] == 0
                and ev["code_1200_count"] == 0
            )
            rows.append(
                {
                    "cell": cell,
                    "pass": p,
                    "exit_code": rc,
                    "instrumented_wall_seconds": round(wall, 3),
                    "slice_complete_ms": ev["slice_complete_ms"],
                    "gcode_sha256": got,
                    "gcode_bytes": got_bytes,
                    "reference_sha256": want,
                    "reference_bytes": want_bytes,
                    "reference_match": ok_ref,
                    "degraded": ev["degraded"],
                    "fatal_error_count": ev["fatal_error_count"],
                    "non_fatal_error_count": ev["non_fatal_error_count"],
                    "code_1200_count": ev["code_1200_count"],
                }
            )
            print(
                f"{cell} pass{p}: exit={rc} wall={wall:.3f}s (attribution only) "
                f"sha={got[:12]} ref={ok_ref} degraded={ev['degraded']} "
                f"c1200={ev['code_1200_count']}",
                flush=True,
            )
            if not ok_clean:
                failures += 1

    doc = {
        "schema": 1,
        "note": (
            "t186 post-t185 instrumented attribution runs (3 passes/cell). "
            "Attribution only; instrumented_wall_seconds is never a wall claim."
        ),
        "identities": identities,
        "runs": rows,
    }
    (HERE / "runs.json").write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
