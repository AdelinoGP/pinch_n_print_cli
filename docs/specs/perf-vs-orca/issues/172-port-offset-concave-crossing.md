# Port Orca's `OffsetConcaveCrossing` into `vendor/clipper2-rust`

Type: task / measured candidate (canonical-parity port + standing gate; needs a
human t81-style ruling before landing)
Label: wayfinder:task
Status: **resolved 2026-11-20** — strict gate PASSED 8/8, landed under the
sixth recorded t81-style exception (granted 2026-11-20 before production edits)
Assignee: 2026-11-20 Antigravity wayfinder session — the human said "Continue
the perf vs orca map" (no ticket named); 172 was first on the frontier (leaving
173 unclaimed).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t170 Clipper2 2.0.1 kernel screen](170-clipper2-2-0-1-kernel-port-screen.md)
([`evidence/t170-clipper2-201-screen/FINDINGS.md`](../evidence/t170-clipper2-201-screen/FINDINGS.md)),
which isolated the remaining offset gap to one missing Orca fork patch, measured
it on three corpora, and priced it.

## Question

**What are the paired 8-cell wall, CPU and output/parity effects of porting
canonical `v2.5.0`'s `OffsetConcaveCrossing` (`clipper.offset.cpp`, an Orca fork
patch not present in upstream Clipper2 2.0.1 or `clipper2-rust` 1.2.0) into
`vendor/clipper2-rust/src/offset.rs`, and does it win `base classic-off` and
`base arachne-off` as the t170 estimate predicts?**

## What is already known (t170)

- **The patch is the whole `f2` factor.** Applying the same ~30-line patch to
  C++ Clipper2 1.5.2 reproduces canonical 2.0.1 **byte-for-byte on 87/87**
  IB + `0.04 mm Miter 3` opening cases. Without it, 2.0.1 is **0.78–0.87x** of
  1.5.2 on those openings. The kernel version contributes nothing.
- **The Rust port without the patch** is **2.38–2.70x** canonical on the two
  canonical shell openings, **2.95–4.57x** on the classic-perimeter offsets
  (`A0/A1/B/C/E/F/G/K`), and **2.57/1.94x** on `IB_DEEP_NEG/POS`. With the patch:
  **1.04–1.30x**, **1.07–1.27x**, and **1.28/1.11x**; vertex totals converge on
  canonical (e.g. `A0` 1.082 → 1.007, `A1` 1.073 → 0.998). The scratch crate
  passes **403/403** of its own tests.
- **The patch text is frozen**:
  [`offset_concave_crossing.patch`](../evidence/t170-clipper2-201-screen/offset_concave_crossing.patch).
  It adds `offset_concave_crossing` (guards: `cos_a <= -0.999`, `sin_a * delta >= 0`,
  the two `4x²` distance tests, `delta_callback` bypass, `path[j] == path[k]`
  bypass) and calls it from `offset_polygon`; the crossing point must be
  **rounded**, not truncated, because C++ `Point64`'s constructor rounds.
- **Whole-cell estimate (post-t169 tree):** central **+16.4 %** on
  `base classic-off` (required 3.9 %), **+15.3 %** on `base arachne-off`
  (required 10.3 %), 5.2–11.4 % on the other six; conservative host-only clears
  the 0.80 % floor in **8/8** (by 5.2x–16.3x). These are two-step estimates, not
  a campaign.
- **The kernel route cannot be the whole route.** Even at zero cost for every
  host and guest offset, six cells stay short of their required cut
  (`base classic-on` 23.3 % vs 52.7 %; `base arachne-on` 21.6 % vs 50.8 %; the
  four benchy cells 10.7–26.1 % vs 39.0–58.5 %). Winning those needs a second
  phase, not a bigger kernel patch.
- **Upstream has no route.** `clipper2-rust` 1.2.0 is still a 1.5.4 port and does
  not contain the patch; there is no 2.x release. A vendor patch is the only
  exact path.
- **Geometry moves.** The patch removes the 3-point concave loops canonical does
  not emit. It is canonical's own algorithm, but it is **not output-neutral**,
  so it needs the human's t81-style exception (t169's fifth exception is the
  precedent; this would be the sixth) and a fresh reference set.

## Scope

1. **Ruling first.** Ask the human for a t81-style exception scoped to
   `vendor/clipper2-rust`'s offset path and its two consumers
   (`classic-perimeters`, `arachne-perimeters`), with the measured corpus
   numbers and the 87/87 identity as the argument. Do not edit production before
   the ruling.
2. **Port.** Apply `offset_concave_crossing.patch` to
   `vendor/clipper2-rust/src/offset.rs`, including the `.round()` conversion.
   Run the crate's own suite and PnP's `slicer-core` offset tests. Add a unit test
   that falsifies the patch: a dense concave inward offset whose output loses
   the 3-point loops and whose vertex count matches the canonical rule; a
   negative control with the patch disabled must fail it.
3. **Output.** Record the 8-cell reference set on both arms; the baseline must
   reproduce t169's set 8/8, and the candidate's deltas must be audited section
   by section (expect the `Internal Bridge` / `Support` directions t169 saw).
4. **Paired gate.** Standing 8-cell protocol, 5 interleaved pairs/cell ordinary
   plus 3/cell accelerated, process CPU corroborating, null band in the same
   window. Pre-declared gate: clears the 0.80 % wall floor in 8/8; no
   supports-off regression; `base classic-off` and `base arachne-off` must beat
   their required cuts if the t170 estimate is right. If either does not, record
   the measured price and hand back — do not weaken the gate.
5. **Disclose the residual.** t170 leaves **1.04–1.30x** per-call Rust-vs-C++
   overhead on the hot sites (rounding/arena differences), and the `DoRound`
   `std::round`-vs-`ceil` Orca patch is **not** included. Neither is required for
   the speed claim; state them.

## Pre-declared gate

- Wall floor 0.80 % in all eight cells, outside the same-window null band,
  process CPU corroborating (never contradicting); **8/8 byte-identical** on the
  accelerated arm vs the ordinary candidate; baseline reproduces t169's active
  set 8/8.
- A new reference set if output moves (it will), recorded per cell.
- The patch is applied only to `vendor/clipper2-rust`; no `polygon_ops` wrapper
  changes other than what the vendor patch requires.

## Acceptance

- Production change + tests green (`cargo test -p slicer-core --features
  host-algos`, the two perimeter modules, `clippy --workspace --all-targets -D
  warnings`, `check-literals`, `build-guests --check` both modes).
- `evidence/t172-offset-concave-crossing/` with the campaign rows, output
  hashes, delta audit, `reference.json`, `FINDINGS.md`, and a checkout-only
  `verify.py --self-test`.
- Map: destination-status block, Decisions-so-far line, updated reference set and
  route arithmetic, and an explicit statement of whether the kernel route is now
  exhausted for the six non-winning cells.

## Answer

**Resolved 2026-11-20: measured candidate, strict Q2 gate PASSED 8/8, landed
under the sixth recorded t81-style exception.** Porting canonical OrcaSlicer
`v2.5.0` (`40ecc66f78`) `clipper.offset.cpp::OffsetConcaveCrossing` (with
`.round()` integer conversion and the AGPLv3 attribution header) into
`vendor/clipper2-rust/src/offset.rs` saves **+14.89 % to +23.01 %** wall across
all eight cells — **5/5 pairs faster in every cell**, **18.6x–28.8x** the 0.80 %
floor, strictly outside every cell's same-window any-pair wall null band — with
process CPU corroborating at **+19.20 % to +30.67 %** (**5/5** in every cell)
and the accelerated arm corroborating at **+14.19 % to +22.63 %** wall /
**+19.33 % to +30.93 %** CPU (**3/3** in every cell).

**Human ruling (2026-11-20, before production edits).**
- **Q1 (sixth recorded t81-style exception):** granted, scoped strictly to
  porting OrcaSlicer `v2.5.0`'s `OffsetConcaveCrossing` (with `.round()` and
  AGPLv3 attribution header) into `vendor/clipper2-rust/src/offset.rs` across
  its downstream host and guest `polygon_ops` offset callers; explicitly
  excludes `DoRound`, `GetLineIntersectPt`, `c2_offset_as_group`, and any other
  `clipper2-rust` or `polygon_ops` changes.
- **Q2 (strict landing gate):** automatic land only if all three hold — 8/8
  clear 0.80 % outside the same-window any-pair wall null band with CPU
  corroborating; 8/8 ordinary/accelerated candidate byte-identity with baseline
  reproducing t169 8/8; and `base classic-off` ≥ 3.9 % + `base arachne-off` ≥
  10.3 % against t169's estimated required cuts. Two-tool OrcaSlicer scoreboard
  kept out of this ticket and graduated as
  [174](174-post-t172-scoreboard-refresh.md).

**Paired results (ordinary, 5 interleaved pairs/cell):**
- `benchy classic-off`: 2.297 → 1.768 s (**+23.01 %**, 5/5, 28.76x floor, null 2.63 %), CPU **+25.70 %** (5/5) — PASS
- `benchy classic-on`: 5.104 → 4.301 s (**+18.41 %**, 5/5, 23.02x floor, null 5.39 %), CPU **+24.83 %** (5/5) — PASS
- `benchy arachne-off`: 2.388 → 1.877 s (**+21.49 %**, 5/5, 26.86x floor, null 13.69 %), CPU **+22.40 %** (5/5) — PASS
- `benchy arachne-on`: 5.277 → 4.319 s (**+17.37 %**, 5/5, 21.72x floor, null 3.46 %), CPU **+23.56 %** (5/5) — PASS
- **`base classic-off`**: 14.187 → 11.233 s (**+21.41 %**, 5/5, 26.77x floor, null 2.29 %, **beats 3.9 % cut by 5.5x**), CPU **+30.67 %** (5/5) — PASS
- `base classic-on`: 53.182 → 45.116 s (**+14.89 %**, 5/5, 18.62x floor, null 5.19 %), CPU **+19.20 %** (5/5) — PASS
- **`base arachne-off`**: 14.814 → 12.255 s (**+19.88 %**, 5/5, 24.85x floor, null 8.95 %, **beats 10.3 % cut by 1.9x**), CPU **+28.77 %** (5/5) — PASS
- `base arachne-on`: 54.900 → 44.315 s (**+15.00 %**, 5/5, 18.75x floor, null 5.51 %), CPU **+19.98 %** (5/5) — PASS

**Output & delta audit.** Baseline reproduces t169's active set **8/8** on both
arms (`16/16`); candidate moves **8/8** and is byte-identical between ordinary
and accelerated arms (**8/8**). New active currency recorded in
[`evidence/t172-offset-concave-crossing/reference.json`](../evidence/t172-offset-concave-crossing/reference.json)
(t169 superseded). Total extruding path shifts by **−0.745 % to +0.007 %** with
no feature loss (`Outer wall` ≤ 0.0012 %, `Inner wall` ≤ 0.042 %; `Support`
−1.57 % benchy / −0.69 % base, reported not rationalised).

**Falsifiers & fixture update.** New unit tests in
`vendor/clipper2-rust/src/offset_tests.rs` were verified red before the patch
(`96 != 32` raw pre-union points on a 32-gon inward offset) and green after
(**405/405** + 3 doc-tests). The pre-patch self-captured fixture
`crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json` was
updated for two inflated rings (`hole_and_island`'s CW hole square rotated by 1
vertex; `dense_ring` shifted ≤ 1 unit = 100 nm on 96/256 concave vertices); all
clipped polyline outputs in `polygon_ops_tdd` were already identical.

**Route arithmetic & kernel-route closure (cross-window estimate, not a ratio of
record).** Composing with t169's estimate against t163's ordinary board puts
**`base classic-off` at ~0.82x (−18.2 %)** and **`base arachne-off` at ~0.89x
(−10.6 %)** — the first two cells in this map projected to beat OrcaSlicer —
while the other six remain above 1.0x (`benchy` ~1.26x / ~1.94x / ~1.44x /
**~1.99x** binding cell at 49.8 % cut; `base` supports-on ~1.80x / ~1.73x).
**The Clipper2 kernel route is now exhausted for the six non-winning cells**
(only 1.04–1.30x per-call Rust-vs-C++ offset residual remains, and `DoRound` is
output-parity only). Next moves: [173](173-support-geometry-boolean-volume.md)
(support-path boolean volume) and [174](174-post-t172-scoreboard-refresh.md)
(fresh two-tool OrcaSlicer 2.5.1 scoreboard to verify 2/8 cells won and
re-baseline the six remaining cuts).

**Evidence:** [`evidence/t172-offset-concave-crossing/FINDINGS.md`](../evidence/t172-offset-concave-crossing/FINDINGS.md)
(`verify.py` PASS, `--self-test` 8/8; `audit_t172.py --self-test` 4/4).
