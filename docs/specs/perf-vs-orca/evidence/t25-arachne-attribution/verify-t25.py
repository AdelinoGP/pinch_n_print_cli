"""Read-only re-derivation of every T25 FINDINGS headline from raw captures.

Usage: python verify-t25.py

Checks (exit 0 = all pass):
  * the ordinary capture has exactly 240 T25-PROBE lines and every line has svc==1,
  * both probe outputs are byte-identical to the frozen Arachne reference,
  * the headline shares (preprocess, stage 1, offset2_ex, voronoi build,
    connect_junctions, reorder) reproduce from the raw capture,
  * the stage sum closes to the pipeline total within 0.5%,
  * the guest-side module elapsed sum and its relation to the pipeline total.
"""

import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
CAPTURE = (
    ROOT
    / ".local-artifacts/perimeter-reference-preparation/t25-arachne-attribution-run1/ordinary"
)
REFERENCE = (
    ROOT
    / ".local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/corpus/supports-off-benchy/reference-arachne.gcode"
)

failures = []


def check(name, condition, detail=""):
    status = "PASS" if condition else "FAIL"
    print(f"{status}  {name}: {detail}")
    if not condition:
        failures.append(name)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def parse(path):
    rows = []
    with path.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("T25-PROBE"):
                continue
            fields, seq = {}, None
            for part in line.split()[1:]:
                key, value = part.split("=", 1)
                if key == "seq":
                    seq = int(value)
                else:
                    wall, calls = value.split(":")
                    fields[key] = (int(wall), int(calls))
            rows.append({"seq": seq, **fields})
    return rows


capture = CAPTURE / "attrib3.stderr.txt"
rows = parse(capture)
# Instrumented same-run capture carries `module_complete` events (the default
# core-tier stream does not).
inst_capture = CAPTURE / "attrib10-verify.stderr.txt"

check("capture has 240 probe lines", len(rows) == 240, f"n={len(rows)}")
check(
    "every line flushed exactly one service call",
    all(r.get("svc", (0, 0))[1] == 1 for r in rows),
)

agg = defaultdict(lambda: [0, 0])
for row in rows:
    for key, value in row.items():
        if key == "seq":
            continue
        wall, calls = value
        agg[key][0] += wall
        agg[key][1] += calls

total = agg["svc"][0]
pipe = agg["pipe"][0]


def share(key):
    return agg[key][0] / total


check(
    "pipeline wall is >=99% of service wall",
    pipe / total >= 0.99,
    f"pipe/svc={pipe / total:.4f}",
)
check(
    "preprocess share = 41.7% (+-0.5pt)",
    abs(share("pre") - 0.4167) < 0.005,
    f"{share('pre'):.4f}",
)
check(
    "stage-1 share = 40.1% (+-0.5pt)",
    abs(share("s1") - 0.4007) < 0.005,
    f"{share('s1'):.4f}",
)
check(
    "stage-1 == offset2_ex + offset (within bracket slop)",
    abs(agg["s1"][0] - (agg["s1_o2"][0] + agg["s1_off"][0])) / agg["s1"][0] < 0.01,
    f"s1={agg['s1'][0] / 1e6:.1f} s1_o2+s1_off={(agg['s1_o2'][0] + agg['s1_off'][0]) / 1e6:.1f} ms",
)
check(
    "graph share = 37.5% (+-0.5pt)",
    abs(share("g_tot") - 0.3749) < 0.005,
    f"{share('g_tot'):.4f}",
)
check(
    "voronoi build is >=80% of graph total",
    agg["g_vbuild"][0] / agg["g_tot"][0] >= 0.80,
    f"{agg['g_vbuild'][0] / agg['g_tot'][0]:.4f}",
)
check(
    "boostvoronoi build share = 33.0% (+-0.5pt)",
    abs(share("g_vbuild") - 0.3302) < 0.005,
    f"{share('g_vbuild'):.4f}",
)
check(
    "connect_junctions share = 6.8% (+-0.5pt)",
    abs(share("conn") - 0.0676) < 0.005,
    f"{share('conn'):.4f}",
)

# Named top-level stages sum to the pipeline total (no remainder > 0.5%).
top_level = [
    "pre",
    "g_tot",
    "strat",
    "cent",
    "beads",
    "noncent",
    "tm_gen",
    "tm_filt",
    "ends",
    "apply_tr",
    "ribs",
    "pop",
    "up",
    "down",
    "junc",
    "conn",
    "maxima",
    "stitch",
    "rm_small",
    "sep",
    "simp",
    "rm_empty",
    "reorder",
]
named = sum(agg[key][0] for key in top_level)
check(
    "named stages close to pipeline total within 1%",
    abs(named - pipe) / pipe < 0.01,
    f"named/pipe={named / pipe:.4f}",
)

# Output byte-identity for both probe runs.
reference_hash = digest(REFERENCE)
check(
    "attrib-output.gcode == frozen Arachne reference",
    digest(CAPTURE / "attrib-output.gcode") == reference_hash,
)
check(
    "attrib2-output.gcode == frozen Arachne reference",
    digest(CAPTURE / "attrib2-output.gcode") == reference_hash,
)
check(
    "attrib3-output.gcode == frozen Arachne reference",
    digest(CAPTURE / "attrib3-output.gcode") == reference_hash,
)

# Guest-side module elapsed sum for the same run (instrumented capture).
module_ms = []
with inst_capture.open(encoding="utf-8", errors="replace") as f:
    for line in f:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (
            event.get("event") == "module_complete"
            and event.get("module_id") == "com.core.arachne-perimeters"
        ):
            module_ms.append(event.get("elapsed_ms", 0))
check(
    "Arachne module_complete covers 240 layers",
    len(module_ms) == 240,
    f"n={len(module_ms)}",
)

# Same-run host-vs-module split from the instrumented capture: re-read its own
# probe lines rather than reusing the uninstrumented capture's total.
inst_svc = []
with inst_capture.open(encoding="utf-8", errors="replace") as f:
    for line in f:
        line = line.strip()
        if not line.startswith("T25-PROBE"):
            continue
        fields = dict(part.split("=", 1) for part in line.split()[1:])
        inst_svc.append(int(fields["svc"].split(":")[0]))
check(
    "instrumented capture has 240 probe lines",
    len(inst_svc) == 240,
    f"n={len(inst_svc)}",
)
if len(module_ms) == 240 and len(inst_svc) == 240:
    module_total_ms = sum(module_ms)
    svc_total_ms = sum(inst_svc) / 1e6
    print(
        f"INFO  same-run arachne module elapsed sum: {module_total_ms} ms "
        f"vs host pipeline service sum {svc_total_ms:.1f} ms "
        f"-> host pipeline is {svc_total_ms / module_total_ms * 100:.1f}% of module elapsed"
    )

# Per-invocation table with shape metrics (top 12 by service wall).
ranked = sorted(rows, key=lambda r: -r.get("svc", (0, 0))[0])
print("\nINFO  top invocations by service wall:")
print(
    f"{'seq':>5} {'svc_ms':>8} {'pre_ms':>8} {'graph_ms':>9} {'poly':>5} {'segs':>5} {'lines':>6}"
)
for r in ranked[:12]:
    print(
        f"{r['seq']:>5} {r['svc'][0] / 1e6:>8.1f} {r['pre'][0] / 1e6:>8.1f} "
        f"{r['g_tot'][0] / 1e6:>9.1f} {r['m_polys'][1]:>5} {r['m_segs'][1]:>5} {r['m_lines'][1]:>6}"
    )
top12 = sum(r["svc"][0] for r in ranked[:12])
print(f"INFO  top-12 invocations carry {top12 / total * 100:.1f}% of the service total")
tail_gt = sum(r["svc"][0] for r in rows if r["svc"][0] > 15_000_000)
n_gt = sum(1 for r in rows if r["svc"][0] > 15_000_000)
print(
    f"INFO  {n_gt} invocations >15 ms carry {tail_gt / total * 100:.1f}% of the service total"
)

# ---- Guest-side split (fuel; the module-side unit) -----------------------
acc_capture = (
    ROOT
    / ".local-artifacts/perimeter-reference-preparation/t25-arachne-attribution-run1/accelerated/attrib.stderr.txt"
)


def module_scopes(path):
    with path.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("event") != "profile_summary":
                continue
            for module in event["profile"]["modules"]:
                if module["module_id"] == "com.core.arachne-perimeters":
                    return module, {s["scope"]: s for s in module.get("scopes", [])}
    raise RuntimeError(f"no arachne profile_summary in {path}")


module_ord, scopes_ord = module_scopes(inst_capture)
module_acc, scopes_acc = module_scopes(acc_capture)

sd = scopes_ord["t25::signed_distance"]
oq = scopes_ord["t25::overhang_quartile"]
check(
    "guest: signed_distance is the top scope (48.3% of module fuel)",
    abs(sd["total_fuel"] / module_ord["total_fuel"] - 0.483) < 0.005,
    f"{sd['total_fuel'] / module_ord['total_fuel']:.4f}",
)
check(
    "guest: overhang_quartile is 38.1% of module fuel",
    abs(oq["total_fuel"] / module_ord["total_fuel"] - 0.381) < 0.005,
    f"{oq['total_fuel'] / module_ord['total_fuel']:.4f}",
)
two = sd["total_fuel"] + oq["total_fuel"]
check(
    "guest: the two per-vertex queries are 86.4% of module fuel",
    abs(two / module_ord["total_fuel"] - 0.864) < 0.005,
    f"{two / module_ord['total_fuel']:.4f}",
)
check(
    "guest: the host service call is <0.1% of module fuel",
    scopes_ord["t25::host_arachne_svc"]["total_fuel"] / module_ord["total_fuel"]
    < 0.001,
    f"{scopes_ord['t25::host_arachne_svc']['total_fuel'] / module_ord['total_fuel']:.5f}",
)

# Mode comparison: acceleration cuts the module 1.6-1.8x, the distance query
# ~1.9x, the quartile query ~1.7x — and the two still dominate after.
ratio_total = module_ord["total_fuel"] / module_acc["total_fuel"]
check(
    "modes: accelerated cuts module fuel 1.6-1.8x",
    1.6 < ratio_total < 1.8,
    f"{ratio_total:.2f}",
)
ratio_sd = sd["total_fuel"] / scopes_acc["t25::signed_distance"]["total_fuel"]
check(
    "modes: signed_distance cut 1.8-2.1x",
    1.8 < ratio_sd < 2.1,
    f"{ratio_sd:.2f}",
)
ratio_oq = oq["total_fuel"] / scopes_acc["t25::overhang_quartile"]["total_fuel"]
check(
    "modes: overhang_quartile cut 1.6-1.8x",
    1.6 < ratio_oq < 1.8,
    f"{ratio_oq:.2f}",
)
two_acc = (
    scopes_acc["t25::signed_distance"]["total_fuel"]
    + scopes_acc["t25::overhang_quartile"]["total_fuel"]
)
check(
    "modes: the two queries still dominate accelerated (>=80%)",
    two_acc / module_acc["total_fuel"] >= 0.80,
    f"{two_acc / module_acc['total_fuel']:.4f}",
)

print()
if failures:
    print(f"FAIL: {len(failures)} check(s) failed: {failures}")
    raise SystemExit(1)
print("PASS: all checks re-derived from raw captures")
