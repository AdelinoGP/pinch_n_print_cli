#!/usr/bin/env python3
"""t170 verify: re-derive every quoted number from the reduced set only.

Usage:  python verify.py [--self-test]

Reads `reduced/*.json` plus the two corpus files they name (both are committed
elsewhere in this repo). Exits 0 when every claim in FINDINGS.md is re-derived
and consistent; `--self-test` must reject all six negative controls.
"""

import copy
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
REPO = HERE.parents[4]
FLOOR_PCT = 0.80

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
TWO_SITES = {
    "SH_OPEN_TOP_CANON",
    "SH_OPEN_BOT_CANON",
    "SH_OPEN_TOP_LEGACY",
    "SH_OPEN_BOT_LEGACY",
}
OFFSET_SITES = {
    "A0",
    "A1",
    "B",
    "C",
    "E",
    "F",
    "G",
    "K",
    "IB_DEEP_NEG",
    "IB_DEEP_POS",
    "IB_CAND_POS",
}
IDENTITY_SITES = {
    "IB_CAND_POS",
    "IB_DEEP_NEG",
    "IB_DEEP_POS",
    "SH_OPEN_BOT_CANON",
    "SH_OPEN_TOP_CANON",
}
LEGACY_SITES = {"SH_OPEN_BOT_LEGACY", "SH_OPEN_TOP_LEGACY"}


def load(name):
    return json.loads((REDUCED / name).read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def aggregate(doc, sites):
    rows = [c for c in doc["cases"] if c["site"] in sites]
    sums = {
        e: round(sum(c[e]["median_ns"] for c in rows) / 1e6, 3)
        for e in ("rust_base", "rust_patched", "v250_decim")
    }
    return rows, sums


def price_cell(cell, inputs, gains):
    x = inputs["cells"][cell]
    self_s = x["self_s"]
    base_self = x["self_baseline_s"]
    wall = x["wall_s"]
    s_off = self_s.get("polygon_ops::offset", 0.0)
    s_two = self_s.get("polygon_ops::offset2_ex", 0.0)
    g_off = x["guest_fuel_s"].get("polygon_ops::offset", 0.0)
    g_two = x["guest_fuel_s"].get("polygon_ops::offset2_ex", 0.0)
    d_off = base_self.get("polygon_ops::offset", 0.0) - s_off
    d_two = base_self.get("polygon_ops::offset2_ex", 0.0) - s_two
    saving_pct = (
        (wall["t169_baseline"] - wall["t169_candidate"]) / wall["t169_baseline"] * 100.0
    )
    k = saving_pct / (d_off + d_two)
    out = {"k": k, "self_s": self_s, "d_off": d_off, "d_two": d_two}
    for label, key in (
        ("conservative", "min"),
        ("central", "central"),
        ("optimistic", "max"),
    ):
        go = gains["offset"][key]
        gt = gains["offset2_ex"][key]
        host = s_off * (1 - 1 / go) + s_two * (1 - 1 / gt)
        guest = g_off * (1 - 1 / go) + g_two * (1 - 1 / gt)
        out[label] = {
            "host_saved_s": round(host, 3),
            "wall_pct_host_only": round(host * k, 2),
            "guest_saved_s": round(guest, 3),
            "wall_pct_host_plus_guest": round((host + guest) * k, 2),
        }
    out["zero_host"] = round((s_off + s_two) * k, 2)
    out["zero_all"] = round((s_off + s_two + g_off + g_two) * k, 2)
    return out


def check(docs, failures):
    f2 = docs["f2"]
    prepass = docs["rust_prepass"]
    t68b = docs["rust_t68_base"]
    t68c = docs["rust_t68_benchy"]
    pricing = docs["pricing"]
    inputs = docs["t169_inputs"]
    inventory = docs["inventory"]
    spread = docs["spread"]

    # 1. Corpus identity and counts.
    for doc, expect in ((f2, 118), (prepass, 118), (t68b, 170), (t68c, 75)):
        corpus = REPO / doc["corpus"]
        if not corpus.is_file():
            failures.append("missing corpus %s" % doc["corpus"])
            continue
        if sha(corpus) != doc["corpus_sha256"]:
            failures.append("corpus hash mismatch %s" % doc["corpus"])
        if len(doc["cases"]) != expect or doc["case_count"] != expect:
            failures.append(
                "%s case count %d/%d != %d"
                % (doc["corpus"], len(doc["cases"]), doc["case_count"], expect)
            )

    # 2. Per-site sums and stored factors re-derive from the cases.
    for doc, name in ((f2, "f2"), (prepass, "prepass"), (t68b, "t68b"), (t68c, "t68c")):
        engines = list(doc["cases"][0].keys())
        engines = [e for e in engines if isinstance(doc["cases"][0][e], dict)]
        by = {}
        for c in doc["cases"]:
            by.setdefault(c["site"], []).append(c)
        for site, items in by.items():
            stored = doc["by_site"][site]
            if stored["n"] != len(items):
                failures.append("%s %s n mismatch" % (name, site))
            for e in engines:
                s = round(sum(x[e]["median_ns"] for x in items) / 1e6, 4)
                if abs(s - stored["sum_ms"][e]) > 5e-3:
                    failures.append(
                        "%s %s %s sum %.4f != %.4f"
                        % (name, site, e, s, stored["sum_ms"][e])
                    )
            if name == "f2":
                sums = stored["sum_ms"]
                checks = {
                    "f1_rust_over_cpp152": sums["rust_pnp"] / sums["cpp_152"],
                    "f2_total_cpp152_over_v250_nodecim": sums["cpp_152"] / sums["v250_nodecim"],
                    "f2_noconcave_cpp152_over_v250_nodecim_noX": sums["cpp_152"] / sums["v250_nodecim_noX"],
                    "f2_patch_v250_nodecim_noX_over_v250_nodecim": sums["v250_nodecim_noX"] / sums["v250_nodecim"],
                    "patch_152_cpp152_over_cpp152_concave": sums["cpp_152"] / sums["cpp_152_concave"],
                }
                for field, got in checks.items():
                    if abs(stored[field] - got) > 2e-3:
                        failures.append("f2 %s %s %.4f != %.4f" % (site, field, stored[field], got))
            if name == "prepass":
                rb = stored["sum_ms"]["rust_base"]
                rp = stored["sum_ms"]["rust_patched"]
                v = stored["sum_ms"]["v250_decim"]
                if abs(stored["patch_gain"] - rb / rp) > 2e-3:
                    failures.append("prepass %s patch_gain" % site)
                if abs(stored["rust_base_over_v250"] - rb / v) > 2e-3:
                    failures.append("prepass %s base/v250" % site)
                if abs(stored["rust_patched_over_v250"] - rp / v) > 2e-3:
                    failures.append("prepass %s patched/v250" % site)

    # 3. Aggregate gain bands, against the stored pricing bands.
    _, prepass_off = aggregate(prepass, OFFSET_SITES)
    _, prepass_two = aggregate(prepass, TWO_SITES)
    _, t68b_off = aggregate(t68b, OFFSET_SITES)
    _, t68c_off = aggregate(t68c, OFFSET_SITES)
    off_gains = [
        prepass_off["rust_base"] / prepass_off["rust_patched"],
        t68b_off["rust_base"] / t68b_off["rust_patched"],
        t68c_off["rust_base"] / t68c_off["rust_patched"],
    ]
    two_gain = prepass_two["rust_base"] / prepass_two["rust_patched"]
    bands = pricing["gain_bands"]
    if abs(bands["offset"]["min"] - min(off_gains)) > 5e-3:
        failures.append(
            "offset gain min %.4f != %.4f" % (bands["offset"]["min"], min(off_gains))
        )
    if abs(bands["offset"]["max"] - max(off_gains)) > 5e-3:
        failures.append(
            "offset gain max %.4f != %.4f" % (bands["offset"]["max"], max(off_gains))
        )
    if (
        not bands["offset"]["min"]
        <= bands["offset"]["central"]
        <= bands["offset"]["max"]
    ):
        failures.append("offset central outside band")
    if abs(bands["offset2_ex"]["central"] - round(two_gain, 3)) > 2e-2:
        failures.append(
            "offset2_ex central %.4f != measured %.4f"
            % (bands["offset2_ex"]["central"], two_gain)
        )
    if (
        not bands["offset2_ex"]["min"]
        <= bands["offset2_ex"]["central"]
        <= bands["offset2_ex"]["max"]
    ):
        failures.append("offset2_ex central outside band")

    # 4. Every pricing cell re-derives from t169_inputs + the bands.
    for cell in CELLS:
        got = price_cell(cell, inputs, bands)
        stored = pricing["cells"][cell]
        if abs(got["k"] - stored["k_pct_per_self_s"]) > 2e-3:
            failures.append(
                "%s k %.4f != %.4f" % (cell, got["k"], stored["k_pct_per_self_s"])
            )
        for label in ("conservative", "central", "optimistic"):
            for field in (
                "host_saved_s",
                "wall_pct_host_only",
                "guest_saved_s",
                "wall_pct_host_plus_guest",
            ):
                if (
                    abs(got[label][field] - stored["patch_price_pct"][label][field])
                    > 1.5e-2
                ):
                    failures.append(
                        "%s %s %s %.4f != %.4f"
                        % (
                            cell,
                            label,
                            field,
                            got[label][field],
                            stored["patch_price_pct"][label][field],
                        )
                    )
        if (
            abs(got["zero_host"] - stored["zero_cost_ceiling_pct"]["host_only"])
            > 1.5e-2
        ):
            failures.append("%s zero host ceiling" % cell)
        if (
            abs(got["zero_all"] - stored["zero_cost_ceiling_pct"]["host_plus_guest"])
            > 1.5e-2
        ):
            failures.append("%s zero all ceiling" % cell)

    # 5. The verdict: exactly two cells win on the central host-only price; the
    #    other six are short even at the zero-cost ceiling.
    winners = [
        c
        for c in CELLS
        if pricing["cells"][c]["patch_price_pct"]["central"]["wall_pct_host_only"]
        >= pricing["cells"][c]["required_cut_pct"]
    ]
    if winners != ["base-classic-off", "base-arachne-off"]:
        failures.append("winner set %r" % winners)
    for cell in CELLS:
        c = pricing["cells"][cell]
        if cell in winners:
            if c["zero_cost_ceiling_pct"]["host_plus_guest"] < c["required_cut_pct"]:
                failures.append("%s winner but ceiling short" % cell)
            if c["patch_price_pct"]["conservative"]["wall_pct_host_only"] < FLOOR_PCT:
                failures.append("%s winner below the 0.80%% floor" % cell)
        else:
            if c["zero_cost_ceiling_pct"]["host_plus_guest"] >= c["required_cut_pct"]:
                failures.append("%s short but ceiling reaches the cut" % cell)
            if (
                c["patch_price_pct"]["central"]["wall_pct_host_only"]
                >= c["required_cut_pct"]
            ):
                failures.append("%s short but central price reaches the cut" % cell)
        # Every cell clears the candidate floor on the conservative host-only price.
        if c["patch_price_pct"]["conservative"]["wall_pct_host_only"] < FLOOR_PCT:
            failures.append("%s does not clear the 0.80%% floor" % cell)

    # 6. f2 decomposition: 1.5.2 + Orca patch is byte-identical to 2.0.1 on all
    #    87 IB/canonical-opening cases, and the kernel alone is slower.
    identity = 0
    identity_total = 0
    legacy_match = 0
    legacy_total = 0
    for c in f2["cases"]:
        if c["site"] in IDENTITY_SITES:
            identity_total += 1
            if c["cpp_152_concave"]["out_hash"] == c["v250_decim"]["out_hash"]:
                identity += 1
        if c["site"] in LEGACY_SITES:
            legacy_total += 1
            if (
                c["cpp_152_concave"]["out_hash"] == c["v250_decim"]["out_hash"]
                and c["cpp_152_concave"]["out_paths"] == 0
            ):
                legacy_match += 1
    if (identity, identity_total) != (87, 87):
        failures.append("patch identity %d/%d != 87/87" % (identity, identity_total))
    if (legacy_match, legacy_total) != (8, 31):
        failures.append(
            "legacy empty-only identity %d/%d != 8/31" % (legacy_match, legacy_total)
        )
    for site in ("SH_OPEN_TOP_CANON", "SH_OPEN_BOT_CANON"):
        s = f2["by_site"][site]
        if not s["f2_noconcave_cpp152_over_v250_nodecim_noX"] < 1.0:
            failures.append("%s kernel-without-patch is not slower than 1.5.2" % site)
        if not s["f2_patch_v250_nodecim_noX_over_v250_nodecim"] > 1.5:
            failures.append("%s 2.0.1 patch factor <= 1.5" % site)
        if not s["patch_152_cpp152_over_cpp152_concave"] > 2.0:
            failures.append("%s 1.5.2 patch factor <= 2.0" % site)

    # 7. Source inventory facts and the patch text.
    if inventory["vendor_clipper2_rust"]["ported_cpp_version"] != "1.5.4":
        failures.append("vendor ported version is not 1.5.4")
    if inventory["vendor_clipper2_rust"]["markers"]["offset_concave_crossing"]:
        failures.append("vendor already has the concave patch")
    if not inventory["canonical_v250"]["markers"]["OffsetConcaveCrossing"]:
        failures.append("canonical lacks OffsetConcaveCrossing")
    if inventory["cpp_152"]["markers"]["OffsetConcaveCrossing"]:
        failures.append("1.5.2 has OffsetConcaveCrossing")
    if inventory["upstream_1_2_0"]["has_offset_concave_crossing"]:
        failures.append("upstream 1.2.0 has the concave patch")
    patch = (HERE / "offset_concave_crossing.patch").read_text(encoding="utf-8")
    for token in (
        "fn offset_concave_crossing",
        "self.offset_concave_crossing(path, j, k, next)",
        "round() as i64",
        "distance_sqr",
    ):
        if token not in patch:
            failures.append("patch missing %r" % token)

    # 8. The two-window spread, so the quoted gains are not a single-run draw.
    w1, w2 = spread["window1"], spread["window2"]
    for key in ("prepass_offset", "t68_base_offset", "t68_benchy_offset"):
        for w in (w1, w2):
            if w[key]["rust_base"] / w[key]["rust_patched"] <= 1.4:
                failures.append("%s gain <= 1.4 in a window" % key)
    if (
        not (
            w1["prepass_two_pass"]["rust_base"] / w1["prepass_two_pass"]["rust_patched"]
        )
        > 1.4
    ):
        failures.append("prepass two-pass gain <= 1.4")


def main():
    docs = {
        "f2": load("f2_ablation.json"),
        "rust_prepass": load("rust_prepass.json"),
        "rust_t68_base": load("rust_t68_base.json"),
        "rust_t68_benchy": load("rust_t68_benchy.json"),
        "pricing": load("pricing.json"),
        "t169_inputs": load("t169_inputs.json"),
        "inventory": load("source_inventory.json"),
        "spread": load("run_spread.json"),
    }
    failures = []
    check(docs, failures)

    if "--self-test" in sys.argv:
        controls = []

        def control(name, mutate):
            bad = copy.deepcopy(docs)
            mutate(bad)
            f = []
            check(bad, f)
            controls.append((name, bool(f), f[0] if f else "not rejected"))

        def drop_case(d):
            d["f2"]["cases"].pop(3)

        control("deleted-case", drop_case)

        def move_price(d):
            d["pricing"]["cells"]["base-classic-off"]["patch_price_pct"]["central"][
                "wall_pct_host_only"
            ] += 1.0

        control("changed-price", move_price)

        def move_self(d):
            d["t169_inputs"]["cells"]["base-classic-off"]["self_s"][
                "polygon_ops::offset"
            ] += 1.0

        control("changed-self-time", move_self)

        def flip_marker(d):
            d["inventory"]["vendor_clipper2_rust"]["markers"][
                "offset_concave_crossing"
            ] = True

        control("flipped-source-marker", flip_marker)

        def move_gain(d):
            d["pricing"]["gain_bands"]["offset"]["min"] = 9.0

        control("changed-gain-band", move_gain)

        def move_required(d):
            d["pricing"]["cells"]["base-classic-on"]["required_cut_pct"] = 5.0

        control("changed-required-cut", move_required)

        rejected = sum(1 for _, ok, _ in controls if ok)
        for name, ok, detail in controls:
            print("%-26s %s" % (name, "REJECTED" if ok else "ACCEPTED: " + detail))
        print("self-test: rejected %d/%d negative controls" % (rejected, len(controls)))
        if rejected != len(controls):
            return 1

    print("claims checked: %d failure(s)" % len(failures))
    for f in failures:
        print("FAIL:", f)
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main())
