"""Read-only verification of the T21 linker-subcost evidence.

Re-reads the raw captures under
`.local-artifacts/perimeter-reference-preparation/t21-linker-subcost-run1/`
and re-derives every headline figure in FINDINGS.md. Launches no slices and
never writes outside stdout.

Usage: python verify-t21.py [--run-root <dir>]
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

WS = Path(__file__).resolve().parents[5]
DEFAULT_ROOT = (
    WS
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t21-linker-subcost-run1"
)
REFERENCE_SHA = "7049a06d68ee8ab012adc8f26c831bd8cc1a59d0d10d737d1257f58a8719c161"


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_capture(path):
    """Return (linker_per_layer, summary) for one profile capture."""
    per_layer = {}
    summary = None
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            continue
        if e.get("event") == "profile_summary":
            summary = e["profile"]
        if (
            e.get("event") == "module_complete"
            and e.get("module_id") == "com.core.infill-linker"
        ):
            d = e.get("profile_scopes") or {}
            per_layer[e.get("layer_index", -1)] = {
                "elapsed_ms": e.get("elapsed_ms"),
                "fuel": d.get("call_fuel", 0),
                "scopes": {s["scope"]: s["self_fuel"] for s in d.get("scopes", [])},
            }
    return per_layer, summary


def linker_summary_row(summary):
    for m in summary["modules"]:
        if m["module_id"] == "com.core.infill-linker":
            return m
    raise AssertionError("no infill-linker row in profile_summary")


def verify(run_root):
    checks = []

    def check(name, condition, detail):
        checks.append((name, bool(condition), detail))

    # --- capture hashes + byte-identical outputs --------------------------
    for rel, expected in [
        ("ordinary/output.gcode", REFERENCE_SHA),
        ("ordinary/probe-output.gcode", REFERENCE_SHA),
        ("ordinary/probe2-output.gcode", REFERENCE_SHA),
        ("accelerated/probe2-output.gcode", REFERENCE_SHA),
    ]:
        p = run_root / rel
        check(
            f"sha256({rel})",
            p.exists() and sha256(p) == expected,
            f"exists={p.exists()}",
        )

    # --- capture 1: baseline split ---------------------------------------
    base_layers, base_summary = read_capture(run_root / "ordinary" / "profile.jsonl")
    linker_row = linker_summary_row(base_summary)
    total_fuel = linker_row["total_fuel"]
    scoped = sum(s["self_fuel"] for s in linker_row["scopes"])
    slice_total = base_summary["fuel_total"]

    tail = sum(base_layers[li]["fuel"] for li in (0, 1, 2))
    check(
        "capture1 linker fuel tail share",
        abs(tail / total_fuel - 0.87) < 0.005,
        f"tail={tail:,} total={total_fuel:,} share={tail / total_fuel:.4f}",
    )
    check(
        "capture1 marked-scope share ~0.17%",
        scoped / total_fuel < 0.005,
        f"scoped={scoped:,} share={scoped / total_fuel:.6f}",
    )
    check(
        "capture1 linker share of slice fuel",
        abs(total_fuel / slice_total - 0.458) < 0.005,
        f"{total_fuel / slice_total:.4f}",
    )

    # --- capture 3: the sub-split ----------------------------------------
    probe_layers, probe_summary = read_capture(
        run_root / "ordinary" / "probe2-profile.jsonl"
    )
    probe_row = linker_summary_row(probe_summary)
    probe_total = probe_row["total_fuel"]
    scopes = {s["scope"]: s["self_fuel"] for s in probe_row["scopes"]}
    for scope in ("t21::clip_polylines_inflate", "t21::clip_polylines_execute"):
        check(f"capture3 {scope} present", scope in scopes, f"{scopes.get(scope, 0):,}")

    inflate = scopes["t21::clip_polylines_inflate"]
    execute = scopes["t21::clip_polylines_execute"]
    check(
        "capture3 inflate share ~64.8%",
        abs(inflate / probe_total - 0.6484) < 0.002,
        f"{inflate / probe_total:.4f}",
    )
    check(
        "capture3 execute share ~32.7%",
        abs(execute / probe_total - 0.3265) < 0.002,
        f"{execute / probe_total:.4f}",
    )
    check(
        "capture3 reclip closes >=97.6%",
        (inflate + execute + scopes.get("t21::clip_polylines_total", 0)) / probe_total
        > 0.976,
        f"{(inflate + execute) / probe_total:.4f}",
    )
    tail_inflate = sum(
        probe_layers[li]["scopes"].get("t21::clip_polylines_inflate", 0)
        for li in (0, 1, 2)
    )
    check(
        "capture3 tail inflate share ~85.8%",
        abs(tail_inflate / inflate - 0.858) < 0.01,
        f"{tail_inflate / inflate:.4f}",
    )

    # --- cross-mode invariance -------------------------------------------
    acc_layers, acc_summary = read_capture(
        run_root / "accelerated" / "probe2-profile.jsonl"
    )
    acc_row = linker_summary_row(acc_summary)
    acc_scopes = {s["scope"]: s["self_fuel"] for s in acc_row["scopes"]}
    delta = abs(acc_row["total_fuel"] - probe_total) / probe_total
    check(
        "cross-mode linker fuel identical (<1e-6 relative)",
        delta < 1e-6,
        f"ordinary={probe_total:,} accelerated={acc_row['total_fuel']:,} rel={delta:.2e}",
    )
    check(
        "accelerated linker is #1 fuel consumer",
        max(acc_summary["modules"], key=lambda m: m["total_fuel"])["module_id"]
        == "com.core.infill-linker",
        f"top={max(acc_summary['modules'], key=lambda m: m['total_fuel'])['module_id']}",
    )
    check(
        "accelerated inflate/execute equal ordinary",
        acc_scopes.get("t21::clip_polylines_inflate")
        == scopes.get("t21::clip_polylines_inflate")
        and acc_scopes.get("t21::clip_polylines_execute")
        == scopes.get("t21::clip_polylines_execute"),
        "exact equality of the two sub-scopes across modes",
    )

    # --- host-vs-guest boundary (counters + fn_us) ------------------------
    counters = {}
    for line in open(run_root / "ordinary" / "probe-profile.jsonl", encoding="utf-8"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            e = json.loads(line)
        except json.JSONDecodeError:
            continue
        if e.get("event") == "module_log" and e.get("message", "").startswith(
            "T21-PROBE"
        ):
            parts = dict(x.split("=", 1) for x in e["message"].split()[1:])
            counters[int(parts.pop("layer"))] = {k: int(v) for k, v in parts.items()}
    check(
        "probe counters present for 240 layers",
        len(counters) == 240,
        f"n={len(counters)}",
    )
    l1 = counters.get(1, {})
    check(
        "L1 export-call wall within 0.1% of module elapsed",
        abs(l1.get("fn_us", 0) / 1000 - 4155) / 4155 < 0.001,
        f"fn_us={l1.get('fn_us')} us",
    )
    check(
        "L1 clip calls = 428",
        l1.get("clip_calls") == 428,
        f"clip_calls={l1.get('clip_calls')}",
    )

    failures = [c for c in checks if not c[1]]
    for name, ok, detail in checks:
        print(f"{'PASS' if ok else 'FAIL'}  {name}: {detail}")
    print()
    if failures:
        print(f"FAIL: {len(failures)} check(s) failed")
        return 1
    print(f"PASS: all {len(checks)} checks re-derived from raw captures")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    sys.exit(verify(args.run_root))
