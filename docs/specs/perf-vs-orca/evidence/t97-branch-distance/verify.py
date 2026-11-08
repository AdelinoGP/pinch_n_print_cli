#!/usr/bin/env python3
"""t97 checkout-only verifier: the branch-distance match is a false premise.

Re-derives every in-repo fact behind
docs/specs/perf-vs-orca/evidence/t97-branch-distance/FINDINGS.md from committed
files only. The canonical source check is external (see REPRODUCE.md); this
script asserts the in-repo facts and the arithmetic that falsify the premise.

Usage:
    python verify.py
    python verify.py --self-test    # five negative controls must be rejected
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
EVIDENCE = ROOT / "docs/specs/perf-vs-orca/evidence"
PLANNER = ROOT / "modules/core-modules/tree-support-planner"


def _appendix_kv(text: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in text.splitlines():
        if " = " in line and not line.lstrip().startswith(";"):
            key, _, value = line.partition(" = ")
            out[key.strip()] = value.strip()
    return out


def check_profiles(classic: dict, arachne: dict) -> None:
    """The matched supports-on profiles set neither branch-distance key."""
    for name, cfg in (("classic", classic), ("arachne", arachne)):
        assert cfg.get("enable_support") is True, f"{name}: supports not enabled"
        assert cfg.get("support_type") == "tree(auto)", f"{name}: not the tree family"
        for key in (
            "tree_support_branch_distance",
            "tree_support_branch_distance_organic",
        ):
            assert key not in cfg, f"{name}: profile sets {key}, premise changes"


def check_appendix(text: str, cell: str) -> None:
    """Orca resolves default->organic; the organic key is 1 mm, normal is 5 mm."""
    kv = _appendix_kv(text)
    assert kv.get("support_type") == "tree(auto)", f"{cell}: not tree(auto)"
    assert kv.get("support_style") == "default", f"{cell}: style is not default"
    assert kv.get("tree_support_branch_distance") == "5", f"{cell}: normal key is not 5"
    assert kv.get("tree_support_branch_distance_organic") == "1", (
        f"{cell}: organic key is not 1"
    )


def check_manifest(text: str) -> None:
    """PnP defaults: normal distance 1.0, style default, organic alias documented."""
    m = re.search(
        r"\[config\.schema\.tree_support_branch_distance\](.*?)(?=\n\[)",
        text,
        re.S,
    )
    assert m and re.search(r"^default\s*=\s*1\.0$", m.group(1), re.M), (
        "branch-distance default is not 1.0"
    )
    s = re.search(r"\[config\.schema\.support_style\](.*?)(?=\n\[)", text, re.S)
    assert s, "support_style schema block missing"
    assert re.search(r'^default\s*=\s*"default"$', s.group(1), re.M), (
        'support_style default is not "default"'
    )
    preamble = text[max(0, s.start() - 500) : s.start()]
    assert "smsTreeOrganic" in preamble, "manifest comment lost the organic mapping"


def check_from_config(text: str) -> None:
    """PnP aliases the canonical-organic default to the classic Strong style."""
    start = text.index("fn from_config")
    end = text.index("/// Whether the config's support family", start)
    body = text[start:end]
    assert '"tree_slim"' in body and "=> Self::Slim" in body, "slim arm missing"
    assert '"tree_strong"' in body and "=> Self::Strong" in body, "strong arm missing"
    assert "_ => Self::Strong" in body, "default/organic alias is not Strong"
    assert "smsTreeOrganic" in body or "smsDefault" in body, (
        "alias comment no longer names the canonical style"
    )


def check_deviation(text: str) -> None:
    """DEV-156 records the organic-engine substitution."""
    rows = [line for line in text.splitlines() if "| DEV-156 |" in line]
    assert len(rows) == 1, "DEV-156 row not found exactly once"
    row = rows[0]
    for needle in ("smsTreeOrganic", "Strong", "organic engine"):
        assert needle in row, f"DEV-156 lost '{needle}'"


def check_t88(text: str) -> None:
    """The t88 arm's numbers, so the ticket quotes them from the source."""
    section = text[text.index("tree_support_branch_distance") :]
    section = section[: section.index("## Candidate pricing")]
    for needle in ("184,115", "176,869", "−3.94", "−0.83"):
        assert needle in section, f"t88 branch-distance section lost '{needle}'"


def check_arithmetic() -> dict:
    """Orca organic 1 mm == PnP 1 mm; the 5 mm arm is 5x Orca's spacing."""
    orca_effective = 1.0
    pnp_effective = 1.0
    arm = 5.0
    assert orca_effective == pnp_effective, "effective distances are not matched"
    assert arm / orca_effective == 5.0, "arm is not 5x Orca's effective spacing"
    return {
        "orca_effective_mm": orca_effective,
        "pnp_effective_mm": pnp_effective,
        "arm_mm": arm,
        "arm_over_orca": arm / orca_effective,
    }


def load_inputs() -> dict[str, object]:
    profiles = {}
    for name in ("classic", "arachne"):
        path = EVIDENCE / f"matched-pair/configs/pnp-{name}-supports-on.json"
        profiles[name] = json.loads(path.read_text(encoding="utf-8"))
    appendices = {}
    for name in ("base-classic", "base-arachne", "benchy-classic", "benchy-arachne"):
        path = EVIDENCE / f"t82-fairness-key-audit/appendix/{name}-supports-on.txt"
        appendices[name] = path.read_text(encoding="utf-8")
    return {
        "profiles": profiles,
        "appendices": appendices,
        "manifest": (PLANNER / "tree-support-planner.toml").read_text(encoding="utf-8"),
        "lib_rs": (PLANNER / "src/lib.rs").read_text(encoding="utf-8"),
        "deviation": (ROOT / "docs/DEVIATION_LOG.md").read_text(encoding="utf-8"),
        "t88": (EVIDENCE / "t88-supports-increment/FINDINGS.md").read_text(
            encoding="utf-8"
        ),
    }


def run_all(inputs: dict[str, object]) -> dict:
    check_profiles(inputs["profiles"]["classic"], inputs["profiles"]["arachne"])
    for cell, text in inputs["appendices"].items():
        check_appendix(text, cell)
    check_manifest(inputs["manifest"])
    check_from_config(inputs["lib_rs"])
    check_deviation(inputs["deviation"])
    check_t88(inputs["t88"])
    return check_arithmetic()


def _expect_reject(label: str, fn, *args) -> None:
    try:
        fn(*args)
    except AssertionError:
        print(f"  negative control rejected: {label}")
        return
    raise SystemExit(f"FAIL: negative control NOT rejected: {label}")


def self_test(inputs: dict[str, object]) -> None:
    c = dict(inputs["profiles"]["classic"])
    c["tree_support_branch_distance"] = 5.0
    _expect_reject(
        "profile sets the normal key", check_profiles, c, inputs["profiles"]["arachne"]
    )

    bad_appendix = inputs["appendices"]["base-classic"].replace(
        "tree_support_branch_distance_organic = 1",
        "tree_support_branch_distance_organic = 5",
    )
    _expect_reject(
        "organic key resolved to 5", check_appendix, bad_appendix, "base-classic"
    )

    bad_manifest = inputs["manifest"].replace(
        '[config.schema.tree_support_branch_distance]\ntype    = "float"\ndefault = 1.0',
        '[config.schema.tree_support_branch_distance]\ntype    = "float"\ndefault = 5.0',
    )
    _expect_reject("manifest default moved to 5.0", check_manifest, bad_manifest)

    bad_lib = inputs["lib_rs"].replace("_ => Self::Strong", "_ => Self::Slim", 1)
    _expect_reject("default alias moved to Slim", check_from_config, bad_lib)

    bad_dev = inputs["deviation"].replace("| DEV-156 |", "| DEV-999 |", 1)
    _expect_reject("DEV-156 row removed", check_deviation, bad_dev)


def main() -> int:
    inputs = load_inputs()
    if "--self-test" in sys.argv:
        self_test(inputs)
        print("SELF-TEST PASS: 5/5 negative controls rejected")
        return 0
    result = run_all(inputs)
    print("PASS: the matched job's effective branch distance is 1 mm on both sides")
    print(f"      {result}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
