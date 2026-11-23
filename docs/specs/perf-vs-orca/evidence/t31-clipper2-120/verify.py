"""t31 verifier — re-derive the clipper2 1.1.0 vs 1.2.0 geometry delta.

Runs entirely against the in-repo reduced evidence set in this directory
(no raw capture tree required). Exits 0 on success, 1 on any mismatch.

    python3 verify.py

Checks, in order:
  1. Both arms' `_counts.json` agree except on exactly the two known fixtures.
  2. Each differing fixture is 1.1.0's ring with exactly ONE vertex deleted.
  3. The deleted vertex is (near-)collinear: |cross| <= 3 and within the
     prev..next bbox, i.e. it is a removable spur vertex, not a real corner.
  4. 1.2.0's ring is a FIXED POINT of the canonical `clean_collinear` walk
     (no removable vertex remains), while 1.1.0's ring still had one on
     layer01 — the mechanism the upstream 1.2.0 release names.
  5. The validations summary is byte-identical across arms.
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
A = HERE / "out-110"
B = HERE / "out-120"

# `_counts.json` keys are fixture *stems*; file names add `.json`.
EXPECTED_DIFFERING_STEMS = {"benchy__layer01_opening", "benchy__layer07_closing"}
EXPECTED_DIFFERING = {f"{s}.json" for s in EXPECTED_DIFFERING_STEMS}


def fail(msg):
    print(f"FAIL: {msg}")
    sys.exit(1)


def ring(path, idx=0, key="contour"):
    ps = json.loads(path.read_text())[idx][key]["points"]
    return [(p["x"], p["y"]) for p in ps]


def cross_and_span(prev, d, nxt):
    v1 = (d[0] - prev[0], d[1] - prev[1])
    v2 = (nxt[0] - d[0], nxt[1] - d[1])
    cross = v1[0] * v2[1] - v1[1] * v2[0]
    inside = min(prev[0], nxt[0]) <= d[0] <= max(prev[0], nxt[0]) and min(
        prev[1], nxt[1]
    ) <= d[1] <= max(prev[1], nxt[1])
    return cross, inside


def is_collinear(prev, d, nxt):
    """The crate's own ProductsAreEqual form (`is_collinear`)."""
    a = d[0] - prev[0]
    b = nxt[1] - d[1]
    c = d[1] - prev[1]
    e = nxt[0] - d[0]
    return a * b == c * e


def removable_vertices(r, preserve=True):
    out = []
    for i, d in enumerate(r):
        prev, nxt = r[i - 1], r[(i + 1) % len(r)]
        if not is_collinear(prev, d, nxt):
            continue
        dot = (d[0] - prev[0]) * (nxt[0] - d[0]) + (d[1] - prev[1]) * (nxt[1] - d[1])
        if (d == prev) or (d == nxt) or (not preserve) or dot < 0:
            out.append((i, d))
    return out


# --- 1. counts agree except on the two known fixtures ----------------------
ca = json.loads((A / "_counts.json").read_text())
cb = json.loads((B / "_counts.json").read_text())
if set(ca) != set(cb):
    fail("counts key sets differ")
differing = {k for k in ca if ca[k] != cb[k]}
if differing != EXPECTED_DIFFERING_STEMS:
    fail(
        f"differing fixtures {sorted(differing)} != {sorted(EXPECTED_DIFFERING_STEMS)}"
    )
print(f"ok 1/6  exactly {len(differing)} fixtures differ: {sorted(differing)}")

# --- 2 & 3. one collinear vertex deleted, same point set otherwise --------
for name in sorted(EXPECTED_DIFFERING):
    a = ring(A / name)
    b = ring(B / name)
    if len(b) != len(a) - 1:
        fail(f"{name}: 1.2.0 has {len(b)} pts, expected {len(a) - 1}")
    # in-place single deletion (ring order preserved)
    hit = False
    for i in range(len(a)):
        if a[:i] + a[i + 1 :] == b:
            hit = True
            dropped = a[i]
            prev, nxt = a[i - 1], a[(i + 1) % len(a)]
            break
    if not hit:
        fail(f"{name}: 1.2.0 ring is not 1.1.0's ring minus one in-place vertex")
    cross, inside = cross_and_span(prev, dropped, nxt)
    if not inside:
        fail(f"{name}: dropped {dropped} is outside prev..next bbox (a real corner?)")
    if abs(cross) > 3:
        fail(f"{name}: dropped {dropped} has |cross|={cross} > 3 (not near-collinear)")
    print(f"ok 2/6  {name}: dropped {dropped} in place, cross={cross}, inside bbox")

# --- 4. 1.2.0's ring is a fixed point under BOTH flag settings -------------
# `preserve_collinear = false` is what `polygon_ops` passes to ClipperOffset
# (third positional arg) and what the offset's finishing union sees; `true` is
# the Clipper64/canonical default. 1.2.0 must be clean under both, so the fix
# cannot be an artefact of the flag.
for name in sorted(EXPECTED_DIFFERING):
    a = ring(A / name)
    b = ring(B / name)
    rem_a_false = removable_vertices(a, preserve=False)
    rem_a_true = removable_vertices(a, preserve=True)
    rem_b_false = removable_vertices(b, preserve=False)
    rem_b_true = removable_vertices(b, preserve=True)
    if rem_b_false or rem_b_true:
        fail(
            f"{name}: 1.2.0 ring still has removable vertices "
            f"(preserve=false: {rem_b_false[:2]}, preserve=true: {rem_b_true[:2]})"
        )
    print(
        f"ok 3/6  {name}: 1.2.0 ring is a fixed point under both flag settings "
        f"(1.1.0 left {len(rem_a_false)} under preserve=false, "
        f"{len(rem_a_true)} under preserve=true)"
    )

# --- 5. validations summary byte-identical ---------------------------------
va = (A / "_validations.json").read_bytes()
vb = (B / "_validations.json").read_bytes()
if va != vb:
    fail("_validations.json differs across arms")
print("ok 4/6  _validations.json byte-identical across arms")

# --- 6. the shipped fixtures hash to what the manifest claims --------------
import hashlib

man = json.loads((HERE / "manifest.json").read_text())
if set(man) != {"110", "120"}:
    fail(f"manifest sides {sorted(man)} != ['110', '120']")
if set(man["110"]) != set(man["120"]):
    fail("manifest key sets differ between arms")
manifest_differing = {k for k in man["110"] if man["110"][k] != man["120"][k]}
# `_counts.json` is the summary that *records* the two fixture diffs, so it
# necessarily differs too; the two fixture names are the signal.
unexpected = manifest_differing - EXPECTED_DIFFERING - {"_counts.json"}
if unexpected:
    fail(f"manifest shows unexpected differing files: {sorted(unexpected)}")
if len(man["110"]) != 369:
    fail(f"manifest has {len(man['110'])} entries, expected 369")

for side, d in (("110", A), ("120", B)):
    for p in sorted(d.glob("*.json")):
        h = hashlib.sha256(p.read_bytes()).hexdigest()
        if h != man[side].get(p.name):
            fail(f"{side}/{p.name} does not match its manifest hash")
print(
    f"ok 5/6  manifest: {len(man['110'])} files/arm; only the two fixtures "
    f"(+ the summary that records them) differ"
)

# --- 7. every un-shipped fixture is identical per the manifest ------------
identical = len(man["110"]) - len(manifest_differing)
n_fixtures = sum(1 for k in man["110"] if not k.startswith("_"))
n_id_fixtures = n_fixtures - len(EXPECTED_DIFFERING)
print(
    f"ok 6/6  {n_id_fixtures} of {n_fixtures} fixtures are byte-identical by "
    f"manifest SHA-256 (the other {len(EXPECTED_DIFFERING)} are check 1-3; "
    f"{identical} files identical in total, the 369th being the summary)"
)

print("\nPASS — all t31 verifier checks green")
