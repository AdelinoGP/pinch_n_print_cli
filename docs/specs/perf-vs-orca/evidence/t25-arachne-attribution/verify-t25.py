"""Read-only re-derivation of every T25 FINDINGS headline from the captures.

Runs against the in-repo reduced captures (`captures/`) by default, so a plain
checkout can reproduce every headline; pass `--capture-root` to point at the
full durable raw tree instead. Never writes outside stdout and launches no
slices.

Checks (exit 0 = all pass):
  * every capture has 240 T25-PROBE lines with svc==1 per line,
  * every measured output is byte-identical to the frozen Arachne reference
    (via `captures/output-hashes.json`),
  * the Finding-1 module/host split per capture,
  * the headline shares (preprocess, stage 1, graph, boostvoronoi,
    connect_junctions) reproduce from the raw capture,
  * the named-stage sum closes to the pipeline total within 1%,
  * the guest-side fuel split and the ordinary/accelerated ratios,
  * the 239/239 per-layer coverage and the top-12 layer share.

Usage: python verify-t25.py [--capture-root <dir>]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from t25_capture import module_events, module_scopes, parse_probe_lines  # noqa: E402

HERE = Path(__file__).resolve().parent
DEFAULT_ROOT = HERE / "captures"

failures = []


def resolve(root: Path, stem: str) -> Path:
    """Accept either the in-repo reduced capture or the full raw capture."""
    for suffix in (".reduced.log", ".stderr.txt"):
        candidate = root / f"{stem}{suffix}"
        if candidate.exists():
            return candidate
    raise SystemExit(f"capture not found: {root / stem}.{{reduced.log,stderr.txt}}")


def load_manifest(root: Path) -> dict:
    manifest = root / "output-hashes.json"
    if not manifest.exists():
        manifest = DEFAULT_ROOT / "output-hashes.json"
    return json.loads(manifest.read_text(encoding="utf-8"))


def check(name, condition, detail=""):
    status = "PASS" if condition else "FAIL"
    print(f"{status}  {name}: {detail}")
    if not condition:
        failures.append(name)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def aggregate(rows):
    agg = defaultdict(lambda: [0, 0])
    for row in rows:
        for key, value in row.items():
            if key == "seq":
                continue
            agg[key][0] += value[0]
            agg[key][1] += value[1]
    return agg


def module_elapsed_ms(path) -> tuple[int, int]:
    events = module_events(path)
    return sum(e.get("elapsed_ms", 0) for e in events), len(events)


def main(root: Path) -> None:
    # ---- probe-line integrity, every capture -----------------------------
    names = [
        "ordinary/attrib3",
        "ordinary/attrib4-inst",
        "ordinary/attrib5-prof",
        "ordinary/attrib10-verify",
        "ordinary/attrib8-final",
        "accelerated/attrib",
    ]
    rows = {}
    for name in names:
        path = resolve(root, name)
        rows[name] = parse_probe_lines(path)
        check(
            f"{name}: 240 probe lines",
            len(rows[name]) == 240,
            f"n={len(rows[name])}",
        )
        check(
            f"{name}: every line flushed exactly one service call",
            all(r.get("svc", (0, 0))[1] == 1 for r in rows[name]),
        )

    # ---- Finding 1: module vs host split, per capture --------------------
    expected = {
        "ordinary/attrib4-inst": (13954, 2741.3),
        "ordinary/attrib5-prof": (14656, 2648.5),
        "ordinary/attrib10-verify": (15033, 2645.2),
        "ordinary/attrib8-final": (20194, 3691.3),
        "accelerated/attrib": (13015, 3268.0),
    }
    for name, (want_ms, want_svc) in expected.items():
        ms, n = module_elapsed_ms(resolve(root, name))
        svc = aggregate(rows[name])["svc"][0] / 1e6
        check(
            f"Finding 1 {name}: module elapsed sum",
            n == 240 and ms == want_ms,
            f"{ms} ms over {n} dispatches",
        )
        check(
            f"Finding 1 {name}: host service sum",
            abs(svc - want_svc) < 0.05,
            f"{svc:.1f} ms",
        )

    # ---- Finding 4: host stage shares (attrib3 is the reduced reference) --
    a = aggregate(rows["ordinary/attrib3"])
    total = a["svc"][0]
    pipe = a["pipe"][0]

    def share(key):
        return a[key][0] / total

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
        abs(a["s1"][0] - (a["s1_o2"][0] + a["s1_off"][0])) / a["s1"][0] < 0.01,
        f"s1={a['s1'][0] / 1e6:.1f} s1_o2+s1_off={(a['s1_o2'][0] + a['s1_off'][0]) / 1e6:.1f} ms",
    )
    check(
        "graph share = 37.5% (+-0.5pt)",
        abs(share("g_tot") - 0.3749) < 0.005,
        f"{share('g_tot'):.4f}",
    )
    check(
        "voronoi build is >=80% of graph total",
        a["g_vbuild"][0] / a["g_tot"][0] >= 0.80,
        f"{a['g_vbuild'][0] / a['g_tot'][0]:.4f}",
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

    # ---- named-stage closure ---------------------------------------------
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
    named = sum(a[key][0] for key in top_level)
    check(
        "named stages close to pipeline total within 1%",
        abs(named - pipe) / pipe < 0.01,
        f"named/pipe={named / pipe:.4f}",
    )

    # ---- output byte-identity (hashes preserved in-repo) ------------------
    manifest = load_manifest(root)
    reference = manifest["frozen_reference"]["sha256"]
    for entry in manifest["captures"]:
        check(
            f"{entry['capture']} output == frozen Arachne reference",
            entry["sha256"] == reference,
            entry["sha256"][:16],
        )

    # ---- Finding 2: guest fuel split (attrib10-verify + accelerated) ------
    module_ord, scopes_ord = module_scopes(resolve(root, "ordinary/attrib10-verify"))
    module_acc, scopes_acc = module_scopes(resolve(root, "accelerated/attrib"))

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

    # ---- Finding 3: mode ratios ------------------------------------------
    ratio_total = module_ord["total_fuel"] / module_acc["total_fuel"]
    check(
        "modes: accelerated cuts module fuel 1.6-1.8x",
        1.6 < ratio_total < 1.8,
        f"{ratio_total:.2f}",
    )
    ratio_sd = sd["total_fuel"] / scopes_acc["t25::signed_distance"]["total_fuel"]
    check(
        "modes: signed_distance cut 1.8-2.1x", 1.8 < ratio_sd < 2.1, f"{ratio_sd:.2f}"
    )
    ratio_oq = oq["total_fuel"] / scopes_acc["t25::overhang_quartile"]["total_fuel"]
    check(
        "modes: overhang_quartile cut 1.6-1.8x", 1.6 < ratio_oq < 1.8, f"{ratio_oq:.2f}"
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

    # ---- per-layer coverage and top-12 share (Finding 2 note) ------------
    layers = []
    for event in module_events(resolve(root, "ordinary/attrib9-verbose")):
        data = event.get("profile_scopes") or {}
        scopes = {s["scope"]: s for s in data.get("scopes", [])}
        layers.append(
            (
                event.get("layer_index"),
                data.get("call_fuel", 0),
                scopes.get("t25::signed_distance", {}).get("calls", 0),
                scopes.get("t25::overhang_quartile", {}).get("calls", 0),
            )
        )
    both = [row for row in layers if row[2] > 0 and row[3] > 0]
    check(
        "layers: 239 of 240 dispatches call both queries",
        len(both) == 239 and len(layers) == 240,
        f"{len(both)}/{len(layers)}",
    )
    total_239 = sum(row[1] for row in layers if row[0] != 0)
    top12 = sum(row[1] for row in sorted(both, key=lambda r: -r[1])[:12])
    check(
        "layers: top 12 carry 17.3% (+-0.5pt) of non-zero-layer fuel",
        abs(top12 / total_239 - 0.173) < 0.005,
        f"{top12 / total_239:.4f}",
    )

    # ---- probe-output evidence (the candidate's measured shares) ---------
    # The two synthetic-ring probe tests are preserved as source + captured
    # stdout; re-derive the quoted ranges from those outputs.
    def probe_shares(stem: str, prefix: str) -> list[float]:
        shares = []
        for run in sorted((HERE / "probe_outputs").glob(f"{stem}.run*.txt")):
            for line in run.open(encoding="utf-8", errors="replace"):
                if line.startswith(prefix):
                    parts = dict(p.split("=", 1) for p in line.split()[1:])
                    shares.append(float(parts["pass1_share"]))
        return shares

    shares = probe_shares("winding_pass_share", "PROBE")
    check(
        "probe: winding pass share lies in 56.5-71.2% over 4 runs x 3 ring sizes",
        len(shares) == 12
        and min(shares) >= 0.565 - 0.001
        and max(shares) <= 0.712 + 0.001
        and min(shares) <= 0.575
        and max(shares) >= 0.702,
        f"min={min(shares):.3f} max={max(shares):.3f} n={len(shares)}",
    )
    scaling = []
    for run in sorted((HERE / "probe_outputs").glob("quartile_shape.run*.txt")):
        for line in run.open(encoding="utf-8", errors="replace"):
            if line.startswith("PROBE"):
                parts = dict(p.split("=", 1) for p in line.split()[1:])
                scaling.append((int(parts["n"]), int(parts["per_call_ns"])))
    sizes = {n for n, _ in scaling}
    check(
        "probe: quartile scaling covers 16..4096 and grows with ring size",
        sizes == {16, 64, 256, 1024, 4096}
        and len(scaling) == 10
        and all(dict(scaling)[16] < dict(scaling)[n] for n in sizes if n != 16),
        f"n={len(scaling)} sizes={sorted(sizes)}",
    )

    print()
    if failures:
        print(f"FAIL: {len(failures)} check(s) failed: {failures}")
        raise SystemExit(1)
    print("PASS: all checks re-derived from the captures")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--capture-root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    main(args.capture_root)
