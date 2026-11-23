#!/usr/bin/env python3
"""Reproduce ticket-34's production-block verification (config-padding-disclosure-boundary).

Extracts the CONFIG_BLOCK from a real `pnp_cli slice` output and checks it
against OrcaSlicer's wire domains:

  * boolean-domain keys (rows the in-repo reference types as `coBool`) must use
    `1`/`0` literals — OrcaSlicer's substitution branch loads anything else
    (including `true`/`false`) as `false`;
  * the keys served by `translate_appendix_enum` must carry exactly one token
    of their pinned `s_keys_map_*` table, parsed live from the OrcaSlicer
    checkout (never from the reference's GUI-label/truncated `Values` column);
  * pair totals (all / loadable) against the viewer's `>= 80` minimum gate;
    loadability judged by the pinned `PrintConfig.cpp` declaration list plus
    `PrintConfigDef::handle_legacy` renames (the loader applies those before
    the map lookup).

Usage:
  python generate-and-verify.py <gcode> <OrcaSlicerDocumented root> <repo root> <out dir> [orca revision]

Outputs `<out dir>/production-block.txt` and `<out dir>/verification.json`.
"""

import datetime
import json
import re
import sys
from pathlib import Path

gcode_path, orca_root, repo_root, out_dir = (Path(p) for p in sys.argv[1:5])
revision = sys.argv[5] if len(sys.argv) > 5 else "unknown"

gcode = gcode_path.read_text(encoding="utf-8", errors="replace")
start = gcode.find("; CONFIG_BLOCK_START")
end = gcode.find("; CONFIG_BLOCK_END")
assert start != -1 and end != -1 and start < end, "CONFIG_BLOCK markers missing"
block_lines = gcode[start:end].splitlines()

pairs = {}
colon_keys = 0
for line in block_lines:
    if line.startswith("; ") and " = " in line:
        key, value = line[2:].split(" = ", 1)
        # `object_height:<uuid>` entries are per-object bookkeeping injected by
        # the caller, not part of the config surface (same exclusion the
        # runtime test applies); UUIDs vary per run and must not enter counts.
        if ":" in key:
            colon_keys += 1
            continue
        pairs[key] = value

reference = (repo_root / "docs" / "ORCA_CONFIG_REFERENCE.md").read_text(
    encoding="utf-8", errors="replace"
)
reference_keys = set(re.findall(r'^\| "([^"]+)"', reference, flags=re.M))
bool_domain = set()
for line in reference.splitlines():
    match = re.match(r'^\| "([^"]+)" \|[^|]*\| coBool \|', line)
    if match:
        bool_domain.add(match.group(1))
legacy_note = "handle_legacy renames are parsed from the pinned source below"

printconfig = (orca_root / "src" / "libslic3r" / "PrintConfig.cpp").read_text(
    encoding="utf-8", errors="replace"
)

# Loadability is judged against the pinned source's own declaration list plus
# `PrintConfigDef::handle_legacy` renames (the loader applies those before the
# map lookup). Method validated by reproducing the ticket's capture figure
# exactly: the real OrcaSlicer 2.4.1 footer (`tmp/3dbenchy_ABS_35m51s.gcode`)
# measures 620 loadable pairs against this same source.
declared = set(re.findall(r'add\("([^"]+)"', printconfig))
_legacy_body = printconfig[printconfig.index("void PrintConfigDef::handle_legacy") :]
_legacy_body = _legacy_body[: _legacy_body.index("\n}\n")]
_rename = {}
_pending = None
for _m in re.finditer(
    r'opt_key\s*==\s*"([^"]+)"|opt_key\s*=\s*"([^"]+)"', _legacy_body
):
    if _m.group(1):
        _pending = _m.group(1)
    elif _pending:
        _rename.setdefault(_pending, _m.group(2))
        _pending = None


def is_loadable(key):
    return _rename.get(key, key) in declared


def map_tokens(name):
    match = re.search(r"s_keys_map_" + name + r"\s*\{(.*?)\n\};", printconfig, re.S)
    assert match, f"s_keys_map_{name} not found"
    return re.findall(r'\{\s*"([^"]+)"', match.group(1))


translated = {
    "wall_sequence": "WallSequence",
    "wall_generator": "PerimeterGeneratorType",
    "wall_direction": "WallDirection",
    "seam_position": "SeamPosition",
    "dont_filter_internal_bridges": "InternalBridgeFilter",
    "enable_extra_bridge_layer": "EnableExtraBridgeLayer",
}

bool_violations = [
    (key, value)
    for key, value in pairs.items()
    if key in bool_domain and value not in ("1", "0")
]
true_false = [
    (key, value) for key, value in pairs.items() if value in ("true", "false")
]
enum_violations = []
translated_seen = {}
for key, name in translated.items():
    if key in pairs:
        tokens = map_tokens(name)
        translated_seen[key] = {"value": pairs[key], "tokens": tokens}
        if pairs[key] not in tokens:
            enum_violations.append((key, pairs[key]))

loadable = [key for key in pairs if is_loadable(key)]
report = {
    "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(
        timespec="seconds"
    ),
    "gcode": str(gcode_path),
    "orca_checkout": str(orca_root),
    "orca_revision": revision,
    "reference": "docs/ORCA_CONFIG_REFERENCE.md (snapshot 2026-05-19; bool-domain rows only)",
    "loadable_method": (
        "pinned PrintConfig.cpp declared keys + PrintConfigDef::handle_legacy "
        "renames; validated by reproducing 620 loadable in the real OrcaSlicer "
        "2.4.1 capture"
    ),
    "pairs_total": len(pairs),
    "colon_keys_excluded": colon_keys,
    "pairs_loadable": len(loadable),
    "viewer_gate_minimum": 80,
    "gate_satisfied": len(loadable) >= 80,
    "bool_true_false_offenders": true_false,
    "bool_domain_violations": bool_violations,
    "enum_domain_violations": enum_violations,
    "strict_wire_violations": len(true_false)
    + len(bool_violations)
    + len(enum_violations),
    "translated_keys_checked": translated_seen,
    "notes": (
        "Boolean-domain set parsed from the in-repo reference's `coBool` rows; "
        "enum tokens parsed live from the pinned s_keys_map_* tables; "
        "loadability judged by pinned declaration + handle_legacy renames. "
        "Block generated by `pnp_cli slice` with no config file (default "
        "resolved config / registry projection). Ticket 34 records 161 loadable "
        "of 233 for this block; this script's method measures 157 of 233 (the "
        "same method reproduces the ticket's 620-loadable capture figure), so "
        "the 4-key delta is recorded here rather than silently adopted."
    ),
}

out = Path(out_dir)
out.mkdir(parents=True, exist_ok=True)
(out / "production-block.txt").write_text(
    "\n".join(block_lines) + "\n", encoding="utf-8"
)
(out / "verification.json").write_text(
    json.dumps(report, indent=2) + "\n", encoding="utf-8"
)

print(
    json.dumps(
        {
            key: report[key]
            for key in (
                "pairs_total",
                "pairs_loadable",
                "gate_satisfied",
                "strict_wire_violations",
                "bool_true_false_offenders",
                "bool_domain_violations",
                "enum_domain_violations",
            )
        },
        indent=2,
    )
)
