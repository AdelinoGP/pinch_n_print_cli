# Post-t158 eight-cell scoreboard: re-derive the ratios of record on the current tree

Type: task / measured scoreboard
Label: wayfinder:task
Status: resolved (2026-11-17; measured board — one interleaved window on the
  current tree gives **1.366x–2.718x** and **0 of 8 won**, required cut
  **26.80 %–63.21 %**, with the two base supports-off cells now the tightest
  (1.366x / 1.429x, inside the 1.5x margin). PnP output is 8/8 identical to
  t151's active set. **The Orca arm itself measured 5.4–24.9 % slower than on
  t145's board in all eight cells**, so cross-window ratio movement is not
  attributable to PnP's work; this table supersedes t145's and t148's as the
  ratios of record and required cuts are no longer comparable across windows.
  See ## Answer)
Assignee: 2026-11-17 OpenCode wayfinder session — the human picked this shape from
  the fog after "Continue the perf vs orca map" (frontier was empty; 1–160
  resolved). Fog item: "The benchy per-layer program is margin, not route (t154)",
  item (b) — "the arithmetic's ledger chain (t145/t148 walls × t150 deltas × this
  take's shares) needs a fresh paired scoreboard before any decision becomes
  sensitive to ±5 %".
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: map fog, "The benchy per-layer program is margin, not route
  (t154)", item (b). Promoted to a ticket on 2026-11-17 by the human's pick.

## Question

The map's ratios of record are a **three-window mixture**, and every phase term
built on them is a chain of deltas rather than a measurement:

- the four **supports-off** cells are t145's board (2026-11-13, `1.853x–3.455x`,
  required cut `46.03 %–71.06 %`);
- the four **supports-on** cells are t148's re-derivation (2026-11-13,
  `2.705x / 2.829x / 3.356x / 3.248x`), taken when the plane-carve fixed the
  support-incompleteness — a different window from t145's;
- since both boards, **five production changes have landed**: t150
  (MeshAnalysis bridge retirement, `−4,445 → 732 ms` base classic-off), t151
  (skirt-hull contour fold, fairness), t157/t158 (`derive_needs_support` exact
  fast path, `KEEP` by human ruling, `4.23–6.37 s` wall on the four base cells
  and `~230 ms` on the benchy cells), plus t142/t143/t135 Stage 2 in between.

So the map currently quotes a projection for its route (`base classic-on
1.0508x → 0.8343x` at canonical prepass cost) and a mixture for its position,
and the destination gate — lower median uninstrumented wall in **every** cell —
cannot be read off either.

**What are the eight cells' matched-settings ratios, on the current tree, in one
interleaved window?**

## Scope

1. **One campaign, one window, both tools.** 8 cells × {Orca, PnP ordinary},
   1 warmup + 3 measured runs per cell, interleaved, matched keys asserted on
   every Orca run by the driver's own fail-closed check. Same rig, fixtures and
   driver as t145 (`evidence/matched-pair/run_scoreboard.ps1`); the fixtures are
   `tmp/3dbenchy.stl` (the 2026-11-12 replacement) and `tmp/base.stl`.
2. **Output identity first.** The tree must reproduce t151's active reference
   set in **8/8** cells — t158 landed byte-identical, so a miss means the tree is
   not the tree the map describes, and the board is not taken.
3. **The table.** Per cell: PnP wall and process CPU medians, Orca wall median,
   the ratio, the **required cut** and the **1.5x cut**, per-cell spread, and the
   box load sample. Both the destination reading (0 of 8 won, or not) and the
   tightest-margin cell named explicitly.
4. **Supersede, don't blend.** This board replaces t145's and t148's as the
   ratios of record; the two prior boards are named as superseded with their
   windows, and no new ratio may be composed from the old ones.
5. **State the phase-attribution limit.** The map's phase shares (prepass
   `63.5–82.4 %`, per-layer `14.4–29.6 %`) are **pre-t158**; the board does not
   re-attribute them, and whatever the board shows about the route arithmetic's
   sensitivity is recorded as a limit or as fog, not as a new projection.

## Pre-declared gate

This is a re-baseline, not a candidate: **no performance gate**. Its correctness
bar is the standing fairness contract:

- PnP output byte-identical to t151's active set in **8/8** cells, every run
  `exit 0`, `degraded=false`, zero exact-Z occupancy diagnostics on the
  supports-on cells (t148's fix);
- every Orca run's resolved-config appendix asserts the matched keys
  (`wall_generator`, `enable_support`, `sparse_infill_density = 20 %`,
  `sparse_infill_pattern = gyroid`, `wall_loops = 2`, layer heights,
  `brim_width = 8`, `brim_type = outer_only`, `skirt_loops = 6`,
  `skirt_distance = 3`, `skirt_height = 1`, `initial_layer_line_width = 0.4`,
  `tree_support_brim_width = 0`, `enable_arc_fitting = disabled`,
  `elefant_foot_compensation = 0`, and the `tree_strong` supports-on pin);
- the accelerated timing arm is attempted once and its result disclosed;
- no causal claim is taken from movement against a prior window.

## Acceptance

- A reduced artifact carrying the 8-cell table, the reference-set identity check
  and the disclosures, plus a `verify.py` that re-derives every quoted number
  from it and exits 0 on a plain checkout.
- The map's destination-status block updated with the measured board, and
  t145/t148 named as superseded ratios of record.
- The next shape named: either the frontier is re-populated from what the board
  shows, or the fog is updated with the route arithmetic's new sensitivity.

## Answer

Resolved 2026-11-17 as a measured board. Evidence:
[`evidence/t161-scoreboard-refresh/`](../evidence/t161-scoreboard-refresh/FINDINGS.md)
(`verify.py` PASS 226/226, `--self-test` 16/16 mutations rejected).

### 1. The board

One window, 8 cells × {Orca, PnP ordinary}, 1 warmup + 3 measured runs per cell,
interleaved. PnP output is **8/8 identical** to t151's active set, every run
`exit 0`, `degraded: false`, zero `exact-Z occupancy` diagnostics, and the
matched keys asserted from all 24 Orca runs.

| cell | PnP | Orca | ratio | required cut | 1.5x cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4.076 s | 2.269 s | 1.796x | 44.31 % | 16.47 % |
| benchy classic-on | 8.144 s | 3.050 s | 2.671x | 62.55 % | 43.83 % |
| benchy arachne-off | 3.790 s | 2.113 s | 1.794x | 44.25 % | 16.37 % |
| benchy arachne-on | 7.337 s | 2.831 s | 2.591x | 61.41 % | 42.12 % |
| base classic-off | 22.603 s | 15.812 s | 1.429x | 30.04 % | **−4.94 %** |
| base classic-on | 70.754 s | 26.030 s | 2.718x | 63.21 % | 44.82 % |
| base arachne-off | 22.505 s | 16.474 s | 1.366x | 26.80 % | **−9.80 %** |
| base arachne-on | 71.377 s | 27.219 s | 2.622x | 61.87 % | 42.80 % |

Range **1.366x–2.718x**, required cut **26.80 %–63.21 %**, **0 of 8 won**. The
tightest cells are now the two base supports-off cells, and both sit inside the
1.5x margin (negative 1.5x cut).

### 2. The finding that constrains the tracker: the Orca binary changed

**The installed OrcaSlicer is 2.5.1**, not the 2.5.0 the map records: the app's
own config header reads `OrcaSlicer 2.5.1`, the install's binaries are dated
**2026-11-13 23:59:44 – 2026-11-14 00:00:44**, and `orca-slicer.exe` is
`5013b1bd…` / 292,160 bytes. **Settings identity did not change** — the vendor
cache stamp is still `2.08.00.16`, so the profile tree and the nine asserted
matched keys are the same — so only the executable moved. t145's board (committed
2026-11-13 00:37) and t148's (04:06) are **2.5.0** numbers; this board is the
first full board on **2.5.1**.

The Orca arm measured **5.4–24.9 % slower in all eight cells** than on t145's
board, and the movement is **efficiency, not work**: Orca's process CPU moved
only −2.2…+11.7 % while its wall moved +5.4…+24.9 %, and **Orca's effective
parallelism (CPU/wall) fell in 8/8 cells** (−2.2…−17.4 %) while **PnP's did not**
(−12.2…+34.4 %, mixed) — a box-wide core shortage would have hit both arms. The
2.5.1 binary is therefore the leading explanation.

**What cannot be separated:** 2.5.0 is no longer installed, so tool change and
machine state are confounded; the post-batch state control (benchy classic-off:
Orca 2.33–2.79 s, PnP 4.76–5.41 s) shows only that the slower behaviour persists
*within* 2.5.1. What is established is that at least that cell's job is
unchanged — its Orca output is 4,077,463 bytes, t145's exact figure, and
byte-constant across runs.

The cell where this matters most is benchy classic-off: PnP rose **+4.2 %** while
Orca rose **+18.5 %**, so its entire −12.1 % ratio movement is Orca-side.
Conversely the two benchy supports-on cells moved **+24.3 % / +24.9 %** on both
arms and their ratios moved **−0.5 % / +0.4 %**, so the paired ratio absorbed the
common shift.

**Rule added:** OrcaSlicer version is a currency boundary (pre-2026-11-13 23:59 =
2.5.0, after = 2.5.1); no ratio and no required cut may be composed across it,
and future campaigns must record the executable's identity — the rig's
profile-stamp gate cannot see a binary swap.

### 3. What is quotable now

- t145's `46.03 %–71.06 %` and t148's supports-on ratios are **superseded** as
  ratios of record — they are 2.5.0 numbers and this table is 2.5.1; required-cut
  arithmetic is not comparable across the tool boundary or across windows.
- The base supports-off improvement is independently corroborated: the landed
  deltas predict `−25.2 %` / `−22.6 %` of t145's PnP wall and the measured
  movement is `−18.9 %` / `−24.0 %`.
- The prepass phase shares and the route projection were **not** re-derived:
  they are pre-t158 and remain cross-window compositions. Recorded as a limit.

### 4. Disclosures

No accelerated timing arm, for two separated reasons: the ordinary-only driver
skips the accelerated freshness gate, and checking that tree afterwards found
**45 stale guests** (the whole accelerated namespace, pre-existing). They were
rebuilt after the campaign (`--accelerated --check` now exit 0, ordinary also
exit 0) and `dist --accelerated` then still fails at exit 1 in `libmimalloc-sys`
(`C1083` on `'\\mimalloc-static.cc'`, MSVC 14.51 verbatim path — the t142
failure), so accelerated output parity is not re-verified here. Also disclosed:
one Orca outlier of 29.807 s on base classic-off run 2 (excluded by being the
largest of three, not dropped); box load 62 samples, min 2 % / median 69.5 % /
max 100 % with saturation samples coinciding with heavy runs; the `tree_strong`
supports-on pin stands; three runs per cell, so no null band is derived.

