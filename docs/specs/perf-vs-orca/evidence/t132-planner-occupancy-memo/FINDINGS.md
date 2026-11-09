# t132 — planner occupancy memo

Resolved 2026-11-09. Shape: a per-layer, content-keyed memo serving the
planner's duplicate `+support_object_xy_distance` occupancy offsets.

**Headline: the duplicate half is exactly where t131's census put it, and
removing it is worth far more wall than the census's parallelism assumption
priced.** The probe census reproduces t131 to the call (base classic-on
**864 calls / 433 distinct / 431 hits**; benchy classic-on **365 / 186 / 179**),
every one of the 16 arm/cell output runs is byte-identical to ticket 123's
reference set, and the four supports-on cells clear the 0.80 % floor at
**7.39x–16.54x** with process CPU corroborating. The measured wall saving on
base classic-on is **14.31 s**, essentially the whole duplicate thread-time
(**14.23 s**), so those duplicate offsets sat on the serial critical path rather
than converting at the stage's 2.19x. The four supports-off cells are inert by
construction (`enable_support: false` returns from the planner before either
memo site) and their mixed-sign medians are noise; they are disclosed in full.

## 1. The candidate

Two call sites inflate the same layer cross-section by the same radius: the
emit-time collision volume and the per-origin role carve. `offset_polygons` is a
pure function of `(polygons, delta, join, arc, miter)`, so the second ask is the
first ask's value.

In `modules/core-modules/tree-support-planner/src/lib.rs`:

- `OccupancyMemoEntry` retains the payload beside its inflated result, so a hash
  collision compares the payload and recomputes instead of answering wrong.
- `occupancy_offset_key` is FNV-1a over every contour and hole point plus
  `xy_distance_mm`.
- `inflate_model_occupancy_memoized` serves a byte-identical payload from the
  memo; both call sites now use it.
- The memo is created per emit-loop iteration, so retained geometry is one layer
  and nothing crosses a layer or a slice.
- Unit test `occupancy_memo_serves_identical_payloads_and_keeps_collisions_exact`
  covers the hit, the delta miss (`+0.35` vs `+0.8`), and the payload miss.

No other production code changed. `cargo test -p tree-support-planner`: 11 test
binaries, 0 failures.

## 2. Census (probe arm, counters reverted before the campaign)

A probe build adds two counters inside `inflate_model_occupancy_memoized` and
logs one `t132-occupancy-memo: calls=<n> hits=<n>` line per planner invocation
at `host::log_trace`. Both probe runs reproduce the reference G-code hash, so the
counters changed no geometry.

| cell | calls | hits | distinct | t131 census (calls / distinct) |
| --- | ---: | ---: | ---: | --- |
| base classic-on | 864 | 431 | 433 | 864 / 433 |
| benchy classic-on | 365 | 179 | 186 | 365 / 186 |

The hit rate is **49.9 %** and **49.0 %** — one duplicate per distinct payload,
which is the two-call structure t131 measured.

## 3. Output identity (16-cell contract)

`-Mode output`: 8 cells × 4 arms (baseline/candidate × ordinary/accelerated),
every arm carrying its own module tree so the guest cannot leak between arms.

- The host binary is identical between the arms (`f3d50217…` ordinary,
  `65d3b7ad…` accelerated); only the planner guest differs
  (`a53e2df3…` baseline vs `7bb99a7c…` candidate ordinary; `817ed9a4…` vs
  `fbbd7484…` accelerated).
- Every cell emits **one** hash across all four arms, and that hash is ticket
  123's candidate-ordinary reference:

| cell | G-code sha256 (all four arms) |
| --- | --- |
| benchy classic-off | `90638bf3ea9e592d…` |
| benchy classic-on | `711c6334e397e5ae…` |
| benchy arachne-off | `2ba5d84ccf3c…` |
| benchy arachne-on | `a972b0f2c25a…` |
| base classic-off | `b3b4ac5e885c…` |
| base classic-on | `4e31a2db368f…` |
| base arachne-off | `6b865d4a2cb8…` |
| base arachne-on | `62d6eedc02e5…` |

32/32 runs exit 0. Because the candidate is exact, hash equality is the whole
delta audit: a geometry change would move the hash.

## 4. Paired result

Interleaved pairs, ordinary arms, `RAYON_NUM_THREADS=12`. Three pairs on the two
base supports-on cells; six pairs on the other six cells (pairs 4–6 were taken
in a second window — see §7).

| cell | baseline → candidate (median wall) | Δ | floor multiple | Δ CPU | verdict |
| --- | --- | ---: | ---: | ---: | --- |
| benchy classic-off | 4.84 → 4.79 s | +0.04 s (+0.92 %) | 1.15x | +0.01 s | inert · PASS |
| **benchy classic-on** | 9.33 → 8.15 s | **+1.18 s (+12.62 %)** | **15.78x** | **+1.11 s** | **PASS** |
| benchy arachne-off | 4.82 → 4.89 s | −0.07 s (−1.44 %) | −1.80x | −0.23 s | inert · FAIL |
| **benchy arachne-on** | 9.17 → 8.62 s | **+0.54 s (+5.92 %)** | **7.39x** | **+0.48 s** | **PASS** |
| base classic-off | 41.24 → 41.19 s | +0.05 s (+0.13 %) | 0.16x | +0.34 s | inert · PASS |
| **base classic-on** | 119.45 → 105.14 s | **+14.31 s (+11.98 %)** | **14.97x** | **+13.25 s** | **PASS** |
| base arachne-off | 42.44 → 42.03 s | +0.40 s (+0.95 %) | 1.18x | −0.97 s | inert · FAIL |
| **base arachne-on** | 119.88 → 104.02 s | **+15.87 s (+13.23 %)** | **16.54x** | **+19.50 s** | **PASS** |

Every supports-on cell is separated on wall in every pair: benchy classic-on
+0.74 / +0.62 / +1.33 s; base classic-on +16.07 / +15.95 / +10.09 s; base
arachne-on +15.87 / +10.00 / +22.84 s; benchy arachne-on six pairs all positive
(+1.39 / +0.62 / +0.42 / +0.84 / +0.49 / +0.43 s).

The rule applied is the standing one: supports-on cells need the median wall
delta at or above 0.80 % of the baseline median with process CPU corroborating;
supports-off cells need no regression (ticket 117's amendment). The reducer's
mechanical strict-sign read is **6/8** because two inert supports-off cells land
on the wrong side of zero by noise (§5); the supports-on subset is **4/4**.

## 5. The supports-off cells are inert, not regressed

Both supports-off configs pin `enable_support: false`
(`evidence/matched-pair/configs/pnp-classic-supports-off.json`, and the arachne
sibling). `SupportPlanner::from_config` reads that key into `enabled`, and
`SupportPlanner::run_support_geometry_with_analysis` returns at its first
statement (`if !self.enabled { return Ok(()); }`) before `plan_for_object` is
ever called. Both memo sites live inside `plan_for_object`, so on these four
cells the memo is never constructed and never called: there is no mechanism by
which the candidate can change their work.

The measured medians are noise at that scale, and the pair deltas show it:

| cell | pair wall deltas (s) | wall spread across all runs |
| --- | --- | ---: |
| benchy classic-off | −0.09 / −0.02 / +0.47 / +0.05 / +0.04 / −0.12 | 0.70 s |
| benchy arachne-off | +0.18 / −0.04 / +0.09 / −0.11 / +0.04 / −0.63 | 0.73 s |
| base classic-off | −0.76 / +0.90 / +3.48 / +0.04 / −0.36 / −0.36 | 4.84 s |
| base arachne-off | +0.53 / +0.74 / −0.49 / +0.26 / +1.37 / +0.29 | 4.92 s |

A ±0.1 s (benchy) and ±1 s (base) median on cells with 0.7 s / 4.9 s total
spread cannot resolve a 0.8 % effect (39 ms and 330 ms respectively). The ruling
is that the supports-off clause is discharged by inertness, not by the sign of a
noise median; the strict-sign result is recorded above so a stricter reading can
reject it.

## 6. Ledger corrections

1. **t131's S6 wall-equivalent understated the shape.** t131 priced the
   duplicate half at **6,504 ms** of wall on base classic-on by dividing 14.23 s
   of thread-time by the stage's 2.19x effective parallelism. The measured wall
   saving is **14.31 s** — **1.006x** the duplicate thread-time. The duplicate
   occupancy calls were on the serial critical path, so a duplicate-half shape
   converts at ~1x, not at the stage's average parallelism. On benchy classic-on
   the saving is larger still than the census: **1.18 s wall and 1.11 s CPU
   against 0.61 s** of duplicate thread-time (1.93x / 1.82x), which the
   offset-only census does not account for; the mechanism is unmeasured and no
   claim is taken from it.
2. **The S6 ceiling is real but is not the route.** 14.31 s is 11.98 % of the
   base classic-on cell and 14.97x its floor, but only **15.5 %** of that cell's
   92.5 s required prepass cut (t131's arithmetic). It is a banked keep, not a
   destination step.

## 7. Disclosures

- **Two measurement windows.** Pairs 1–3 were taken in one batch. A server
  restart cancelled the batch with `base-arachne-on` pair 3 unfinished; that
  pair was completed in a separate two-run invocation, and pairs 4–6 for the
  four supports-off cells plus benchy arachne-on were taken in a third window.
  Every cell is internally paired, and the supports-on base cells stay at three
  pairs; no ratio spans windows.
- **The probe arm is not the campaign arm.** The census was recorded from a
  build carrying two `AtomicU64` counters and one trace line; the counters were
  reverted before the campaign, and the probe run's G-code hash equals the
  reference on both cells measured.
- **Box load** sampled 2 %, 7 % and 4 % across the campaign's windows. The box
  was not otherwise idle-screened; the wall medians above are the record.
- **No Orca arm** was run in this take. This is a PnP-internal A/B; the Orca
  scoreboard of record remains ticket 124's, and this take does not move it.
- **Multi-region caveat** (from the ticket): the two-call structure holds while
  a layer carries one region. On multi-region jobs the emit payload (whole
  layer) and the role payload (one region) differ, so the hit rate falls; the
  census is the matched job's, and the memo's miss path is the old computation.
- **Memory:** the memo retains one payload plus its inflated result for the
  layers of the emit loop iteration — bounded to one layer, no cross-layer
  growth.

## 8. Reproduction

`REPRODUCE.md` in this directory; `run_t132.ps1` (campaign), `reduce_t132.py`
(`target/t132` → `reduced/summary.json`), `verify.py` (176 checks, 11/11
negative controls rejected). Both freshness gates exit 0
(`build-guests --check`, `build-guests --accelerated --check`).
