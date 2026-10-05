#!/usr/bin/env python3
"""Verify the ticket-46 frozen-job drift boundary claims from tracked files only.

Reads census.json + summary.json + the visual bundles and re-checks:

  * the three era references re-derived this take (frozen 7049a06d,
    t44 1b71f83d, current 5a61a106) appear with the recorded bytes;
  * the byte arithmetic (current = t44 + fans - alias) holds;
  * the t47 references (11e7d38a t44-job, e1088b3d classic-off) are recorded
    and the alias-line reconstruction claim is stated;
  * every era completed clean (status ok, no fatal/non-fatal, not degraded);
  * the visual manifests exist with the recorded hashes and the request files
    match the manifests' source paths.

Exits 0 when every check passes; nonzero and prints FAIL lines otherwise.
This verifier requires no raw G-code tree; census.json is the reduced form.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

T47_T44JOB = "11e7d38a60f5018badd9c06424c296fcfc12e5d87bc785800fa2c803ba555a58"
T47_CLASSIC = "e1088b3df78b51b4d0add0c64b3cf143d44ca95cfb5c15db14080772b4e77fbf"
T44_REF = "1b71f83db378d33c2c52ee84028a7180b8b65d26edffde7eff0b06b40b2d403b"
FROZEN_REF = "7049a06d68ee8ab012adc8f26c831bd8cc1a59d0d10d737d1257f58a8719c161"
CURRENT_SHA = "5a61a1066a4607c6968cc038eb221a38ed39d30b5d26d8448ad87895abc92c4a"

fails: list[str] = []


def check(cond: bool, label: str) -> None:
    if not cond:
        fails.append(label)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


census = json.loads((HERE / "census.json").read_text(encoding="utf-8"))
summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
eras = census["eras"]

# --- era references ---------------------------------------------------------
frozen = eras["frozen_t38_snapshot"]
t44 = eras["t44_snapshot"]
current = eras["current_head"]

check(frozen["gcode_sha256"] == FROZEN_REF, "frozen reference is not 7049a06d")
check(t44["gcode_sha256"] == T44_REF, "t44 reference is not 1b71f83d")
check(current["gcode_sha256"] == CURRENT_SHA, "current-head sha is not 5a61a106")
check(frozen["bytes"] == 3_821_661, "frozen bytes != 3821661")
check(t44["bytes"] == 3_806_297, "t44 bytes != 3806297")
check(current["bytes"] == 3_809_027, "current bytes != 3809027")
check(current["fan_lines"] == 277, "current fan lines != 277 (275 M106 + 2 M107)")
check(t44["fan_lines"] == 0, "t44 fan lines != 0 (pre-repair)")
check(frozen["fan_lines"] == 0, "frozen fan lines != 0")

# --- byte arithmetic --------------------------------------------------------
arith = census["byte_arithmetic"]
check(
    current["bytes"] == t44["bytes"] + arith["fan_bytes"] - arith["alias_line_bytes"],
    "size arithmetic current == t44 + fans - alias does not hold",
)
check(arith["fan_bytes"] == 2760, "recorded fan bytes != 2760")
check(arith["alias_line_bytes"] == 30, "recorded alias-line bytes != 30")
check(
    arith["t47_t44job_reference_sha256"] == T47_T44JOB,
    "t47 t44-job reference not recorded",
)
check(
    arith["t47_classic_off_reference_sha256"] == T47_CLASSIC,
    "t47 classic-off reference not recorded",
)
check(arith["t44_reference_sha256"] == T44_REF, "t44 reference not recorded")
check(arith["frozen_reference_sha256"] == FROZEN_REF, "frozen reference not recorded")

# --- clean completion on every era -----------------------------------------
for name, era in eras.items():
    completion = era.get("completion")
    check(completion is not None, f"{name}: completion event missing")
    if completion:
        check(completion["status"] == "ok", f"{name}: status != ok")
        check(completion["degraded"] is False, f"{name}: degraded true")
        check(completion["fatal_error_count"] == 0, f"{name}: fatal errors")
        check(completion["non_fatal_error_count"] == 0, f"{name}: non-fatal errors")

# --- containment negative control ------------------------------------------
for name, era in eras.items():
    c = era["sparse_containment"]
    check(
        c["violating_points"] == 0,
        f"{name}: {c['violating_points']} sparse points outside the wall bbox",
    )

# --- summary.json (human-readable, written alongside) -----------------------
check(summary.get("frozen_reference_sha256") == FROZEN_REF, "summary: frozen ref")
check(summary.get("t44_reference_sha256") == T44_REF, "summary: t44 ref")
check(summary.get("current_reference_sha256") == CURRENT_SHA, "summary: current ref")
check(
    summary.get("t47_t44job_plus_alias_reconstruction") is True,
    "summary: t47 recon flag",
)
check(
    summary.get("t47_classic_plus_alias_reconstruction") is True,
    "summary: t47 classic flag",
)

# --- visual bundles ---------------------------------------------------------
vis = HERE / "visual"
for name, meta in summary["visual_manifests"].items():
    mpath = vis / name / "manifest.json"
    check(mpath.exists(), f"visual manifest missing: {name}")
    if mpath.exists():
        check(
            sha256(mpath) == meta["manifest_sha256"],
            f"visual manifest hash drifted: {name}",
        )
        req = json.loads((vis / meta["request_file"]).read_text(encoding="utf-8"))
        check(
            req["source"]["path"].replace("\\", "/").endswith(meta["gcode_path_leaf"]),
            f"request source does not name the expected gcode: {name}",
        )

if fails:
    print(f"FAIL ({len(fails)}):")
    for f in fails:
        print("  -", f)
    sys.exit(1)
print("PASS: ticket-46 drift boundary census verified (exit 0)")
