#!/usr/bin/env python3
"""Verify t162's claims from the in-repo reduced artifact only.

Stdlib-only. Exits 0 when every check passes, 1 otherwise.
`--self-test` replays mutations and requires each to be rejected.
"""

import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
SUMMARY = os.path.join(HERE, "reduced", "parity8.json")
REF = os.path.join(
    REPO,
    "docs",
    "specs",
    "perf-vs-orca",
    "evidence",
    "t151-skirt-hull-contour",
    "reference.json",
)

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


def check(summary, ref):
    ok = 0
    fail = []

    def want(cond, msg):
        nonlocal ok
        if cond:
            ok += 1
        else:
            fail.append(msg)

    want(summary.get("ticket") == 162, "ticket id is not 162")
    proto = summary.get("protocol", {})
    want(proto.get("cells") == 8, "protocol.cells != 8")
    want(
        proto.get("tools") == ["pnp-ordinary", "pnp-accelerated"],
        "protocol.tools is not the two PnP arms",
    )
    want(proto.get("runs_per_cell") == 1, "protocol.runs_per_cell != 1")

    dist = summary.get("dist", {})
    want(dist.get("exit_code") == 0, "dist exit code is not 0")
    want(dist.get("modules_staged") == 24, "dist did not stage 24 modules")
    want(dist.get("module_diagnose_pass") is True, "module diagnose did not pass")
    want(
        dist.get("module_provenance") == "external", "module provenance is not external"
    )

    rows = summary.get("rows", [])
    want(len(rows) == 16, "expected 16 rows (8 cells x 2 tools), got %d" % len(rows))
    seen = set()
    for r in rows:
        key = (r.get("cell"), r.get("tool"))
        want(key not in seen, "duplicate row %r" % (key,))
        seen.add(key)
        cell = r.get("cell")
        want(cell in CELLS, "unknown cell %r" % (cell,))
        want(
            r.get("tool") in ("pnp-ordinary", "pnp-accelerated"),
            "unknown tool %r" % (r.get("tool"),),
        )
        want(r.get("exit_code") == 0, "%s/%s exit code not 0" % key)
        want(r.get("degraded") == "false", "%s/%s degraded" % key)
        want(
            r.get("reference_sha256") == ref.get(cell),
            "%s/%s reference hash does not match reference.json" % key,
        )
        want(
            r.get("sha256") == ref.get(cell),
            "%s/%s hash does not equal the active reference" % key,
        )
        want(r.get("matches_reference") is True, "%s/%s match flag false" % key)
    want(
        seen == {(c, t) for c in CELLS for t in ("pnp-ordinary", "pnp-accelerated")},
        "the 8 cells x 2 tools matrix is not complete",
    )
    want(summary.get("all_match") is True, "all_match is not true")
    return ok, fail


def main():
    if not os.path.exists(SUMMARY):
        sys.exit("missing reduced artifact: %s" % SUMMARY)
    summary = json.load(open(SUMMARY, encoding="utf-8"))
    ref = json.load(open(REF, encoding="utf-8"))["active_set"]

    if "--self-test" in sys.argv:
        ok, fail = check(summary, ref)
        if fail:
            print("FAIL (baseline artifact is already inconsistent):")
            for f in fail:
                print("  -", f)
            return 1
        mutations = []

        def mut(name, fn):
            s = copy.deepcopy(summary)
            fn(s)
            mutations.append((name, s))

        mut("hash", lambda s: s["rows"][0].__setitem__("sha256", "0" * 64))
        mut(
            "match_flag", lambda s: s["rows"][3].__setitem__("matches_reference", False)
        )
        mut("all_match", lambda s: s.__setitem__("all_match", False))
        mut("drop_row", lambda s: s["rows"].pop())
        mut("duplicate_row", lambda s: s["rows"].append(copy.deepcopy(s["rows"][0])))
        mut("exit_code", lambda s: s["rows"][2].__setitem__("exit_code", 1))
        mut("degraded", lambda s: s["rows"][5].__setitem__("degraded", "true"))
        mut(
            "reference_hash",
            lambda s: s["rows"][7].__setitem__("reference_sha256", "f" * 64),
        )
        mut("dist_exit", lambda s: s["dist"].__setitem__("exit_code", 1))
        mut("dist_modules", lambda s: s["dist"].__setitem__("modules_staged", 23))
        mut(
            "dist_provenance",
            lambda s: s["dist"].__setitem__("module_provenance", "integrated"),
        )
        mut("tools", lambda s: s["protocol"].__setitem__("tools", ["pnp-ordinary"]))

        rejected = 0
        for name, s in mutations:
            _, fails = check(s, ref)
            if fails:
                rejected += 1
            else:
                print("SELF-TEST FAILURE: mutation %r was accepted" % (name,))
        print("self-test: %d/%d mutations rejected" % (rejected, len(mutations)))
        return 0 if rejected == len(mutations) else 1

    ok, fail = check(summary, ref)
    if fail:
        print("FAIL: %d/%d checks" % (len(fail), ok + len(fail)))
        for f in fail:
            print("  -", f)
        return 1
    print("PASS: %d/%d checks" % (ok, ok))
    return 0


if __name__ == "__main__":
    sys.exit(main())
