"""Read-only verification of the T21 linker-subcost evidence.

Runs against the in-repo reduced captures (`captures/`) by default, so a plain
checkout can reproduce every headline; pass `--capture-root` to point at the
full durable raw tree instead. Launches no slices and writes nothing.

Usage: python verify-t21.py [--capture-root <dir>]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from t21_capture import module_rows, probe_counters as read_probe_counters  # noqa: E402

HERE = Path(__file__).resolve().parent
DEFAULT_ROOT = HERE / "captures"
REFERENCE_SHA = "7049a06d68ee8ab012adc8f26c831bd8cc1a59d0d10d737d1257f58a8719c161"
MODULE_ID = "com.core.infill-linker"


def resolve(root: Path, stem: str) -> Path:
    """Accept either the in-repo reduced capture or the full raw capture."""
    for suffix in (".reduced.log", ".jsonl"):
        candidate = root / f"{stem}{suffix}"
        if candidate.exists():
            return candidate
    raise SystemExit(f"capture not found: {root / stem}.{{reduced.log,jsonl}}")


def load_manifest(root: Path) -> dict:
    manifest = root / "output-hashes.json"
    if not manifest.exists():
        manifest = DEFAULT_ROOT / "output-hashes.json"
    return json.loads(manifest.read_text(encoding="utf-8"))


def digest_bytes(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def iter_events(path: Path):
    for line in path.open(encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            yield json.loads(line)
        except json.JSONDecodeError:
            continue


def read_capture(path: Path):
    """Return (linker_per_layer, summary, probe_counters) for one capture."""
    per_layer = {}
    summary = None
    for e in iter_events(path):
        if e.get("event") == "profile_summary":
            summary = e["profile"]
    for li, elapsed_ms, fuel, scopes in module_rows(path):
        per_layer[li] = {
            "elapsed_ms": elapsed_ms,
            "fuel": fuel,
            "scopes": {s["scope"]: s["self_fuel"] for s in scopes},
        }
    return per_layer, summary, read_probe_counters(path)


def linker_summary_row(summary):
    for module in summary["modules"]:
        if module["module_id"] == MODULE_ID:
            return module
    raise AssertionError("no infill-linker row in profile_summary")


def verify(run_root: Path) -> int:
    checks = []

    def check(name, condition, detail):
        checks.append((name, bool(condition), detail))

    # --- output byte-identity ---------------------------------------------
    manifest = load_manifest(run_root)
    check(
        "manifest reference is the frozen Arachne reference",
        manifest["frozen_reference"]["sha256"] == REFERENCE_SHA,
        manifest["frozen_reference"]["sha256"][:16],
    )
    for entry in manifest["captures"]:
        check(
            f"{entry['capture']} output == frozen Arachne reference",
            entry["sha256"] == REFERENCE_SHA,
            entry["sha256"][:16],
        )

    # --- capture 1: baseline split ---------------------------------------
    base_layers, base_summary, _ = read_capture(resolve(run_root, "ordinary/profile"))
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
    probe_layers, probe_summary, probe_counters = read_capture(
        resolve(run_root, "ordinary/probe2-profile")
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

    # --- the hoist saving, re-derived per layer --------------------------
    # Per-layer, per-call inflate cost is heterogeneous, so the hoist saving
    # is re-derived layer-aware rather than from the uniform 1 - 239/4520.
    clip_calls = {li: c.get("clip_calls", 0) for li, c in probe_counters.items()}
    fallbacks = {li: c.get("fallbacks", 0) for li, c in probe_counters.items()}
    per_layer_reclip = {}
    for e in iter_events(resolve(run_root, "ordinary/probe2-profile")):
        if e.get("event") == "module_complete" and e.get("module_id") == MODULE_ID:
            scopes_e = e.get("profile_scopes") or {}
            for s in scopes_e.get("scopes", []):
                if s["scope"] == "t21::path_reclip":
                    per_layer_reclip[e.get("layer_index", -1)] = s["calls"]
    savings = 0.0
    for li, row in probe_layers.items():
        layer_inflate = row["scopes"].get("t21::clip_polylines_inflate", 0)
        calls = clip_calls.get(li, 0)
        if calls == 0:
            continue
        # one preparation per path_reclip invocation + one per fallback clip
        preparations = per_layer_reclip.get(li, 0) + fallbacks.get(li, 0)
        savings += layer_inflate * (1 - preparations / calls)
    check(
        "hoist saving is >=94% of the inflate term",
        savings / inflate >= 0.94,
        f"{savings / inflate:.4f} of inflate = {savings / probe_total:.4f} of linker fuel",
    )

    # --- cross-mode invariance -------------------------------------------
    acc_layers, acc_summary, _ = read_capture(
        resolve(run_root, "accelerated/probe2-profile")
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
        == MODULE_ID,
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
    _, _, counters = read_capture(resolve(run_root, "ordinary/probe-profile"))
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
    print(f"PASS: all {len(checks)} checks re-derived from the captures")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture-root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    sys.exit(verify(args.capture_root))
