# t151 — Skirt hull input: the brim area's contour, not the bead centreline

Candidate: `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`)
folds the **brim area's outer contour** into the skirt hull instead of the
emitted brim beads' centreline. Closed by measurement on 2026-11-14.

## 1. What changed

Two lines of geometry, one of them a move:

```rust
// after  area = outer - inner [- support_area]
for ex in &area {
    brim_points.extend(ex.contour.points.iter().map(slicer_ir::Point2::to_mm));
}
```

and the old fold — `brim_points.extend(path.points.iter().map(|p| (p.x, p.y)))`
inside the `tryExPolygonOffset` emit loop — is deleted. Nothing else in the
module moved: the t144 anchor reconstruction (`offset(filled, 0.5 × modal
outer-wall width)`), the 0.1 mm `brim_object_gap`, the canonical flow-spacing
quantisation, the support clip, the `skirt_height > 1` layer-hull offsets, and
the skirt ring formula `skirt_distance + spacing/2 + i × spacing` are untouched.

## 2. Why this is the canonical input

Canonical `Print::_make_skirt` (`Print.cpp`) appends
`m_objectBrimAreasByInstance[...]` `area.contour.points` — the **contour** of
each brim area, holes excluded — to the skirt group's `occupied_points`, beside
the object hull built from `layer->lslices` contours. `make_brim` (`Brim.cpp`)
fills that map with `brimAreaMap`, which `outer_inner_brim_area` populates after
the `no_brim_area` subtraction; so the canonical input is the brim area's outer
boundary, post-support-clip, exactly the `area` the module already holds after
`clip_polygons(&outer, &inner, Difference)` and the support difference.

PnP folded `path.points` of the emitted rings instead. The outermost emitted
ring is `offset(area, −spacing/2)`, i.e. it sits half a flow spacing inside the
area contour, so the whole skirt ring landed 0.1786 mm too far in.

## 3. Falsifier added

`skirt_hull_folds_the_brim_area_contour_not_the_bead_centreline`
(`modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`) pins the analytic
value: a 1.0 mm half-extent square, 0.4 mm line width, 8 mm brim and no
`first_layer_height` put the area contour at `1.2 + 0.1 + 8.0 = 9.3` mm and the
single skirt loop `3.0 + 0.2` mm outside it, so `x_max = 22.5`; a centreline hull
reads `22.3`. Verified both ways in this take: with the change the test passes at
`22.5`, and with `lib.rs` stashed it fails with
`outermost skirt x=22.3 should be 22.5` — the assertion names both values and
cannot pass on the old code.

## 4. Output campaign: 24 runs, 8 cells × 3 arms

One host binary (`a830ff1e…`) for all three arms; only the `skirt-brim` guest
differs (`7618955c…` baseline → `d690e527…` candidate ordinary → `d52b6ebe…`
candidate accelerated component). A hash diff over the 24-module trees confirms
`skirt-brim/skirt-brim.wasm` is the **only** differing artifact between the
baseline and candidate ordinary arms. Every run exited 0.

| cell | baseline (t148 active) | candidate (ordinary = accelerated) | bytes |
| --- | --- | --- | ---: |
| benchy classic-off | `1ae0cfa8…` | `3d28d1b3…` | −162 |
| benchy classic-on | `37788518…` | `eb202973…` | +163 |
| benchy arachne-off | `8faf02b1…` | `39ad6289…` | +33 |
| benchy arachne-on | `f86d8897…` | `77c3c578…` | −357 |
| base classic-off | `c0b9014f…` | `9bb18f0e…` | +177 |
| base classic-on | `eccbd523…` | `eccbd523…` (**inert**) | 0 |
| base arachne-off | `17f63e93…` | `2f249cdb…` | +41 |
| base arachne-on | `1811425f…` | `1811425f…` (**inert**) | 0 |

**The baseline reproduces the active reference set in 8/8.** The candidate is
byte-identical across the ordinary and accelerated arms in **8/8** and moves in
**6/8**.

**The ticket's "all eight hashes move" is falsified, and the mechanism is the
t144 one:** on the two base supports-on cells the first-layer support footprint
clips the *whole* brim area, so `area` is empty, `brim_points` stays empty, and
the skirt hull has no brim input to change. The delta audit proves that
inertness line-by-line (§5): zero moved sections and zero changed `E` lines,
which is stronger than hash equality. The support clip is out of scope
(t138/t139) and untouched.

**The measured radial move is the canonical half-spacing on every moving cell.**
Skirt bbox growth (mm, min-x / max-x / min-y / max-y) and the brim bbox, which
must not move:

| cell | skirt bbox Δ | brim bbox | brim→skirt step |
| --- | --- | --- | --- |
| benchy classic-off | −0.178 / +0.179 / −0.178 / +0.178 | unchanged | 4.965 → **5.144** |
| benchy arachne-off | −0.178 / +0.179 / −0.179 / +0.178 | unchanged | 4.963 → **5.142** |
| base classic-off | −0.179 / +0.178 / −0.180 / +0.179 | unchanged | 4.962 → **5.140** |
| base arachne-off | −0.179 / +0.178 / −0.179 / +0.178 | unchanged | 4.964 → **5.142** |
| benchy classic-on | −0.178 / 0 / 0 / 0 | unchanged | (support-dominated side) |
| benchy arachne-on | −0.178 / 0 / 0 / 0 | unchanged | (support-dominated side) |

The brim-to-skirt step is `max_x(Skirt) − max_x(Brim)`; on the two benchy
supports-on cells the +x hull extreme is set by support geometry rather than the
brim, so only the −x side responds and the step metric is not brim-driven there.
Against t146's record — Orca **5.139 mm** measured, **5.143 mm** predicted — the
candidate closes to **5.140–5.144 mm**, and the brim bbox is untouched in every
cell, which is the control that the brim itself did not move.

## 5. Delta audit: `Skirt`-only in 6/6, inertness proved in 2/2

`audit_delta.py` compares every section of the baseline and candidate G-code with
the extrusion field stripped and reports the `E` profile separately.

- **`Skirt` is the only moved section in the six moving cells**, and the two base
  supports-on cells have **no moved section at all**.
- **Outside `Skirt`, every motion line is identical with `E` stripped in 8/8.**
- **The ticket's literal "`|dE|` outside `Skirt` is zero" clause is falsified and
  disclosed:** 3–35 lines per moving cell (≤ 0.007 % of 95,859–508,651
  outside-Skirt motion lines) carry a re-rounded `E`, max **0.00098 mm** on base
  classic-off, 0.00025 mm on benchy. The mechanism is the running extrusion
  accumulator re-rounding after the skirt's length changes — t144 measured the
  same effect at the same 0.00098 mm magnitude — and the deltas are *not* a
  constant offset per section (they vary in sign and magnitude), so this is
  rounding, not moved geometry. The two inert cells change **zero** `E` lines.

## 6. Adhesion oracle: 4/4 supports-off, and every residual shrinks to the brim-extent term

`oracle_skirt_brim.py` against the Orca reference run for the same window
(`-Tools orca -Runs 1 -Warmups 0`), band 3.0 % on the skirt outer-loop area:

| supports-off cell | baseline | candidate | t146 estimate |
| --- | ---: | ---: | ---: |
| benchy classic-off | 1.32 % | **0.05 %** | — |
| benchy arachne-off | 1.34 % | **0.04 %** | — |
| base classic-off | 2.89 % | **0.37 %** | ~0.4 % |
| base arachne-off | 3.45 % (MISS) | **0.94 %** | ~0.9 % |

**4/4 pass, up from 3/4**; base Arachne-off flips from a 3.45 % miss to a 0.94 %
pass, and base classic-off's 0.11-point headroom becomes 2.63 points. The brim
swept-area rel-diffs are **unchanged in all four cells** (1.79 / 1.73 / 1.14 /
1.86 %), which is the second control that the brim did not move. PnP's skirt
outer-loop area rises 2268.47 → 2300.03 mm² (benchy-off, Orca 2298.88) and
603.64 → 619.31 mm² (base-off, Orca 621.60).

The residual is now what t146's decomposition predicted it would be: the ~0.10 mm
brim-extent term (inside its 12 % band) plus ≤ 0.04 mm of Arachne width, with the
0.1786 mm skirt-hull term gone.

## 7. Freshness and gates

```
cargo test -p skirt-brim                                     19 passed (1 new)
cargo xtask build-guests                                     rebuilt 1 guest (skirt-brim-guest)
cargo xtask build-guests --check                             exit 0
cargo xtask build-guests --accelerated --check               exit 0
```

## 8. Paired A/B: attribution only, and every median is inside the measured null band

3 interleaved pairs per cell × 8 cells × 2 ordinary arms = **48 runs**, all exit
0, every pair carrying the same hash on both arms (`1ae0cfa8…` / `37788518…` /
`8faf02b1…` / `f86d8897…` / `c0b9014f…` / `eccbd523…` / `17f63e93…` /
`1811425f…` on the baseline side, and the candidate's own set across arms). The
null control is the **same arm on both sides**, 3 cells × 3 pairs:

| | wall | CPU |
| --- | ---: | ---: |
| null band (9 pairs, any-pair absolute) | **12.86 %** | **6.98 %** |
| A/B median, range over the 8 cells | −4.70 … +6.43 % | −0.99 … +4.17 % |

**No wall claim is taken or possible:** every A/B median is inside the measured
null band, and the pair-level spread reaches **+49.77 %** on one base
Arachne-on pair. The window was **not quiet** and this is disclosed rather than
smoothed: the absolute walls are roughly twice the output campaign's on the same
box (base classic-on 166.77 s median against 82.74 s in the output run; base
Arachne-off 63.97 s against 29.01 s), so only paired deltas are read at all, and
the null band was measured on the same busy box — which is why it is ±12.86 %. The
surface being changed is t103's **~235 ms** skirt-brim module-reported
whole-print time, far below the instrument's noise on this window.

## 9. Disposition and disclosures

**Verdict: KEEP — fairness fix landed, oracle 3/4 → 4/4, no wall claim, no route
change.** The skirt hull now folds the canonical brim-area contour; the matched
job moves 6/8 cells, the brim is provably untouched, and the last supports-off
oracle miss is closed.

- **Reference set.** A fresh 16-cell output set was recorded and is written to
  [`reference.json`](reference.json) as the active set for later output tickets;
  t148's set is retained there as `superseded_set` because it is exactly what the
  baseline arm reproduced in this campaign. The two base supports-on hashes are
  unchanged from t148 **by measurement**, not by assumption.
- **Pre-declared items falsified and disclosed, not widened.** (a) "all eight
  hashes move" — six do; the two base supports-on cells cannot, because the
  support clip empties the brim area, and the audit proves zero moved sections
  and zero changed `E` lines there. (b) "`|dE|` outside `Skirt` is zero" — it is
  ≤ 0.00098 mm on 3–35 of up to 508,651 lines, the accumulator re-rounding t144
  measured at the same magnitude.
- **Drive-by, disclosed.** The pre-existing red recorded by t142,
  `integrated_parity_skirt_brim_tdd`, was **not caused** by this change: its
  fixture omitted two keys the module declares (`brim_object_gap`,
  `min_skirt_length`), so `from_config` failed closed before any geometry ran.
  Adding both to the fixture (registry defaults 0.1 / 0.0) revives the test, which
  now passes and compares native against wasm finalization on the changed module.
- **Scope.** Module-only, no `SliceIR`/WIT change, no new clipper operation, no
  host-source change; read as inside t140's exception ("the layer-0 object
  footprint and the `skirt_height` layer hulls"). Both freshness gates exit 0.
- **No scoreboard re-derivation** (the surface is ~235 ms); t145's post-swap
  scoreboard stands with its own disclosures. No accelerated **timing** arm (the
  `libmimalloc-sys` host-link failure, unchanged since t142); accelerated
  **output** parity was checked and is identical in 8/8.
- **Oracle scope:** supports-off only. The four supports-on cells remain
  support-engine-limited (t138/t139); with 4/4 supports-off the overall adhesion
  oracle is **4/8**.
