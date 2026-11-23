#!/usr/bin/env python3
"""Mechanical join for the t82 fairness-key parity audit.

Inputs (all repo-relative, no gitignored raw data):
  appendix/*.txt                      OrcaSlicer resolved-config appendices, one per cell
  docs/ORCA_CONFIG_REFERENCE.md       upstream key metadata, tab/subsection headings
  docs/specs/orca-feature-gap/issues/01-asset-gap-inventory.md
                                      per-key verified `live` presence flag
  docs/15_config_keys_reference.md    PnP runtime key registry (generated tables)
  ../../matched-pair/configs/*.json   matched-job PnP overrides

Output:
  key-table.csv    one row per appendix key, no filtering: the audit's
                   completeness artifact
  summary.json     counts re-derived by verify.py

This script makes **no** judgement about geometry relevance. It only joins the
sources and computes value equality. The curated verdict is merged from
`verdicts.json`, so the mechanical join stays reproducible without it.

Notes on the sources:
  * `docs/ORCA_CONFIG_REFERENCE.md` ends with an alphabetical cross-reference
    index that repeats every key with no section. Only the categorised body is
    read (everything before that index).
  * A PnP key can be declared by several modules with different defaults
    (`classic-perimeters` and `arachne-perimeters` are separate manifests).
    Every declaration is kept; a key with conflicting defaults is flagged
    `ambiguous` rather than silently taking the last row.
"""

from __future__ import annotations

import csv
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
assert (REPO / "docs" / "ORCA_CONFIG_REFERENCE.md").is_file(), REPO

APPENDIX = HERE / "appendix"
REFERENCE = REPO / "docs" / "ORCA_CONFIG_REFERENCE.md"
INVENTORY = (
    REPO
    / "docs"
    / "specs"
    / "orca-feature-gap"
    / "issues"
    / "01-asset-gap-inventory.md"
)
PNP_KEYS = REPO / "docs" / "15_config_keys_reference.md"
MATCHED_CONFIGS = HERE.parent / "matched-pair" / "configs"

CELLS = [
    "benchy-classic-supports-off",
    "benchy-classic-supports-on",
    "benchy-arachne-supports-off",
    "benchy-arachne-supports-on",
    "base-classic-supports-off",
    "base-classic-supports-on",
    "base-arachne-supports-off",
    "base-arachne-supports-on",
]


def read_appendix(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        m = re.match(r"^\s*;?\s*([a-z0-9_]+)\s*=\s*(.*?)\s*$", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def parse_reference(path: Path) -> dict[str, tuple[str, str]]:
    """key -> (tab, subsection) from the categorised body of the reference."""
    out: dict[str, tuple[str, str]] = {}
    tab = ""
    sub = ""
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("## "):
            tab = line[3:].strip()
            if tab.lower().startswith("cross-reference"):
                break
            sub = ""
        elif line.startswith("### "):
            sub = line[4:].strip()
        else:
            m = re.match(r'^\|\s*"([a-z0-9_]+)"\s*\|', line)
            if m:
                out.setdefault(m.group(1), (tab, sub))
    assert out, "reference parsed empty"
    return out


def parse_inventory(path: Path) -> dict[str, tuple[str, str, str]]:
    """key -> (section, col_flag, live_flag) from the verified gap inventory."""
    out: dict[str, tuple[str, str, str]] = {}
    section = ""
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("### "):
            section = line[4:].strip()
        else:
            m = re.match(
                r"^\|\s*`([a-z0-9_]+)`\s*\|\s*(\S+)\s*\|\s*(\S+)\s*\|\s*$", line
            )
            if m:
                out.setdefault(m.group(1), (section, m.group(2), m.group(3)))
    return out


GENERATED_BEGIN = re.compile(r"^<!-- BEGIN GENERATED: (.+?) ")


def parse_pnp_keys(path: Path) -> dict[str, list[dict[str, str]]]:
    """key -> every declaration {type, default, source} in the generated tables."""
    out: dict[str, list[dict[str, str]]] = {}
    block = ""
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        m = GENERATED_BEGIN.match(line)
        if m:
            block = m.group(1)
            continue
        if line.startswith("<!-- END GENERATED"):
            block = ""
            continue
        if not block:
            continue
        row = re.match(
            r"^\|\s*`([a-z0-9_]+)`\s*\|\s*([^|]+?)\s*\|\s*`([^`]*)`\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*`?([^|`]*?)`?\s*\|$",
            line,
        )
        if row:
            out.setdefault(row.group(1), []).append(
                {
                    "type": row.group(2).strip(),
                    "default": row.group(3).strip(),
                    "range": row.group(4).strip(),
                    "base": row.group(5).strip(),
                    "source": row.group(6).strip(),
                    "block": block,
                }
            )
    assert out, "PnP registry parsed empty"
    return out


def parse_matched_overrides() -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    for path in sorted(MATCHED_CONFIGS.glob("pnp-*.json")):
        out[path.stem[len("pnp-") :]] = {
            k: str(v) for k, v in json.loads(path.read_text(encoding="utf-8")).items()
        }
    return out


def norm(value: str) -> tuple[str, str]:
    """Normalise a resolved scalar so equal values compare equal.

    Orca writes booleans as 0/1 and percentages as `20%`; PnP's generated
    registry renders strings with Rust-debug quotes. Semantic equivalence
    beyond this (enums, percents-of-what) is the adjudicator's job.
    """
    v = (value or "").strip()
    if len(v) >= 2 and v[0] == '"' and v[-1] == '"':
        v = v[1:-1]
    if v in ("0", "false", "False"):
        return ("bool", "false")
    if v in ("1", "true", "True"):
        return ("bool", "true")
    pct = v.endswith("%")
    if pct:
        v = v[:-1]
    try:
        f = float(v)
    except ValueError:
        return ("str", v.lower())
    return ("pct" if pct else "num", f"{f:.6g}")


def main() -> int:
    appendices = {cell: read_appendix(APPENDIX / f"{cell}.txt") for cell in CELLS}
    keys = sorted(set().union(*[set(a) for a in appendices.values()]))

    reference = parse_reference(REFERENCE)
    inventory = parse_inventory(INVENTORY)
    pnp = parse_pnp_keys(PNP_KEYS)
    overrides = parse_matched_overrides()

    rows = []
    for key in keys:
        values = {cell: appendices[cell].get(key, "") for cell in CELLS}
        distinct = sorted(set(values.values()))
        constant = len(distinct) == 1
        orca_value = distinct[0] if constant else ""

        tab, sub = reference.get(key, ("", ""))
        inv_section, inv_col, inv_live = inventory.get(key, ("", "", ""))

        decls = pnp.get(key, [])
        if not decls:
            pnp_status = "absent"
            pnp_defaults = ""
            pnp_modules = ""
            pnp_types = ""
        else:
            defaults = sorted({d["default"] for d in decls})
            pnp_status = (
                "ambiguous"
                if len({norm(d["default"]) for d in decls}) > 1
                else "declared"
            )
            pnp_defaults = " | ".join(defaults)
            pnp_modules = " | ".join(sorted({d["source"] for d in decls}))
            pnp_types = " | ".join(sorted({d["type"] for d in decls}))

        override_values = sorted({ov[key] for ov in overrides.values() if key in ov})
        # PnP's resolved value for the comparison cell: a matched-config
        # override if one exists, else the single declared default.
        if override_values:
            effective = (
                override_values[0]
                if len(override_values) == 1
                else " | ".join(override_values)
            )
        else:
            effective = decls[0]["default"] if len(decls) == 1 else ""

        relation = ""
        if decls:
            if not constant:
                per_cell = []
                for cell in CELLS:
                    matches = [norm(d["default"]) == norm(values[cell]) for d in decls]
                    per_cell.append(any(matches))
                relation = "equal-per-cell" if all(per_cell) else "differs-per-cell"
            else:
                matches = [norm(d["default"]) == norm(orca_value) for d in decls]
                if all(matches):
                    relation = "equal"
                elif any(matches):
                    relation = "equal-some"
                else:
                    relation = "differs"

        rows.append(
            {
                "key": key,
                "orca_constant": "yes" if constant else "no",
                "orca_value": orca_value,
                "orca_values_by_cell": "" if constant else " ".join(distinct),
                "reference_tab": tab,
                "reference_section": sub,
                "inventory_section": inv_section,
                "inventory_col": inv_col,
                "inventory_live": inv_live,
                "pnp_status": pnp_status,
                "pnp_defaults": pnp_defaults,
                "pnp_modules": pnp_modules,
                "pnp_types": pnp_types,
                "pnp_overrides": " | ".join(override_values),
                "value_relation": relation,
            }
        )

    with (HERE / "key-table.csv").open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)

    summary = {
        "cells": CELLS,
        "appendix_keys_per_cell": {cell: len(appendices[cell]) for cell in CELLS},
        "union_keys": len(keys),
        "constant_across_cells": sum(1 for r in rows if r["orca_constant"] == "yes"),
        "varying_across_cells": sum(1 for r in rows if r["orca_constant"] == "no"),
        "pnp_declared": sum(1 for r in rows if r["pnp_status"] == "declared"),
        "pnp_ambiguous": sum(1 for r in rows if r["pnp_status"] == "ambiguous"),
        "pnp_absent": sum(1 for r in rows if r["pnp_status"] == "absent"),
        "shared_equal": sum(
            1 for r in rows if r["value_relation"] in ("equal", "equal-per-cell")
        ),
        "shared_equal_some": sum(
            1 for r in rows if r["value_relation"] == "equal-some"
        ),
        "shared_differs": sum(
            1 for r in rows if r["value_relation"].startswith("differs")
        ),
        "not_in_reference": sum(1 for r in rows if not r["reference_tab"]),
        "not_in_inventory": sum(1 for r in rows if not r["inventory_section"]),
    }
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
