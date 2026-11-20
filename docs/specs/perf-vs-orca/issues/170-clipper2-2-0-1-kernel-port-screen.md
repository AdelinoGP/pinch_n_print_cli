# Clipper2 1.1.0 → 2.0.1 kernel upgrade screen for `vendor/clipper2-rust`

Type: task / attribution + screen (kernel upgrade, no blind port)
Label: wayfinder:task
Status: **resolved 2026-11-20** — the kernel premise is falsified; the gap is one
missing Orca fork patch, which is graduated as a candidate (t172). No production
edit.
Assignee: 2026-11-20 OpenCode wayfinder session — the human said "Continue the
perf vs orca map" (no ticket named); 170 was first on the frontier (leaving 171
unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t167 canonical ClipperUtils replay](167-canonical-clipperutils-replay.md)
([`evidence/t167-canonical-clipperutils-replay/FINDINGS.md`](../evidence/t167-canonical-clipperutils-replay/FINDINGS.md)),
whose 3-way ablation isolated a `1.72x–1.76x` (`f2`) non-decimated speedup from Clipper2 `1.5.2 → 2.0.1` + wrapper on canonical `0.04 mm Miter 3` shell openings and a `1.20x–1.47x` (`f1`) Rust `1.1.0 → C++ 1.5.2` residual, widening under 12-way concurrent load.

## Question

**After edge decimation and flat-path `offset2_ex` ([t169](169-canonical-offset-edge-decimation-and-flat-opening.md)) are accounted for, what specific algorithmic and data-structure changes between `vendor/clipper2-rust` 1.1.0 and canonical `v2.5.0`'s C++ Clipper2 2.0.1 (`D:\slicerProject\Orca(pnp_gui)\deps_src\clipper2`, commit `222c6a2df5`) drive the remaining `~2.1x–2.5x` gap on canonical `0.04 mm Miter 3` shell openings (`f1 * f2 = 1.40 * 1.72 = 2.41x` bot, `1.41 * 1.76 = 2.48x` top) and the wider 12-way loaded slowdown (`1.54x–1.75x` Rust vs `1.27x–1.46x` canonical), and what is the cheapest exact port path?**

## What is already known (t167)

- **Clipper1 is ruled out:** canonical `v2.5.0` deleted Clipper1 in commit `222c6a2df5` (*"Improve performance by migrating to Clipper2 2.0.1 (#15969)"*), `orca_v2_5_0` beats `orca_clipper1` on `offset` (`0.78x` sum / `0.47x` median), `base-t68sites` (`0.45x`), `union` (`0.79x`), and canonical `SH_OPEN_*_CANON` (`2.45–2.57x` vs `2.02–2.06x`), and Clipper1 has a hairline-sliver inversion bug (`IB_DEEP_NEG` L133 & L493).
- **Non-decimated gap on canonical `0.04 mm Miter 3` shell openings:** in t167's 3-way ablation (`ablation_decimation.json`), setting `ShortestEdgeLength = 0.0` still leaves `rust_pnp / orca_v2_5_0_nodecim` at **2.41x** (`SH_OPEN_BOT_CANON`) and **2.48x** (`SH_OPEN_TOP_CANON`), splitting into:
  - `f1` (`rust_pnp` 1.1.0 → `cpp_c2_1_5_2`): **1.40x–1.41x**
  - `f2` (`cpp_c2_1_5_2` → `orca_v2_5_0_nodecim` 2.0.1 + `ClipperUtils`): **1.72x–1.76x**
  - Note: on `SH_OPEN_*_LEGACY` (`0.20 mm Round`), `f2` is `0.92x–0.98x` — the `1.72x–1.76x` jump is **specific to Miter `opening_ex` / `c2_offset_expolygons` / flat pass-1 `Paths64` / `PreserveCollinear(false)` / Clipper2 2.0.1 `ClipperOffset`**.
- **12-way concurrent scaling gap:** under 12-way concurrency, `rust_pnp` slows down `1.54x–1.75x` vs `1.27x–1.46x` for `orca_v2_5_0` on `base-t68sites` and `SH_OPEN_*_CANON`, suggesting larger allocation / memory-footprint working sets in `vendor/clipper2-rust` 1.1.0.

## What t169 settled (2026-11-20)

- **Flat-path pass-1 `offset2_ex` was already production** (landed 2026-07-11
  `a5433f93`, extracted into `offset2_ex_first_pass` by `b56f429a`), so this
  screen's premise "after edge decimation and flat-path `offset2_ex` are
  accounted for" is now: **Arm 1 banked before the map, Arm 2 banked by t169.**
- **`append_offset_path` decimation is ported and measured**
  ([t169](169-canonical-offset-edge-decimation-and-flat-opening.md)): the ring
  harvest now runs at `0.005 * |delta|` in `offset_paths` and at
  `|delta2| * 0.005` in `offset_paths_flat` for `offset2_ex`'s second pass.
  Whole-cell wall fell **+21.91…+25.08 %** on the four `base` cells and
  **+5.74…+15.21 %** on the four benchy cells; `polygon_ops::offset` self wall
  fell **41.0–55.5 %** and `offset2_ex` **12.6–25.1 %** on the same tree.
- **So the remaining canonical offset gap is `f1` plus the 2.0.1 kernel diff**:
  t167's `f1` (`rust_pnp` 1.1.0 → C++ 1.5.2) is **1.40x–1.41x** on
  `SH_OPEN_*_CANON`, and `f2`'s non-decimation half
  (`1.5.2 → 2.0.1` + wrapper, `1.72x–1.76x`) is what this screen is for. The
  decimation factor `f3` is no longer available as a lever — it is in the tree.
- **Two things the port did not take**, both pre-existing and out of scope here
  but worth pricing alongside a kernel upgrade: canonical's
  `c2_offset_as_group` per-path fallback (`ClipperUtils.cpp`) and
  `c2_offset_setup`'s `MiterLimit >= 2` / `ArcTolerance <= |delta| / 4` clamps.
  Neither is touched by t169; if a 2.0.1 port lands without them, the wrapper
  remains non-canonical even when the kernel matches.

## Scope

1. **Diff & micro-ablate `f2` (`1.72x–1.76x`) on `SH_OPEN_*_CANON`:** separate (a) flat-path pass-1 `offset2_ex` (no intermediate `PolyTree64`), (b) `PreserveCollinear(false)` in `Clipper64` / `ClipperOffset`, (c) `Clipper2 1.5.2 → 2.0.1` diff in `clipper.offset.cpp` and `clipper.engine.cpp` (`git show 222c6a2df5 -- deps_src/clipper2/`), and (d) `c2_offset_expolygons` single-group offsetting.
2. **Audit `vendor/clipper2-rust` 1.1.0 vs Clipper2 2.0.1:** identify which 2.0.1 optimizations (e.g., `ClipperOffset` miter/group handling, active-edge/outpt arena layout, `PreserveCollinear`, redundant allocations) are missing from `vendor/clipper2-rust`, plus whether `clipper2-rust` has an upstream 2.x release or whether targeted patches to `vendor/clipper2-rust` capture ≥ 80% of `f1 * f2`.
3. **Price the ceiling on the post-t169 tree** (or on t167's replay harness + `ShellClassification` Pass 1) and graduate a concrete implementation ticket if the remaining ceiling clears the 0.80 % gate across the matrix (or close as sub-floor if t169's flat-path `offset2_ex` + decimation already captures the bulk of `f2`).

## Acceptance

- `evidence/t170-clipper2-201-screen/` with clean sub-factor replay attribution on `t167`'s `base-prepass-t167.corpus` + `base-t68sites.corpus`, source diff inventory of Clipper2 `1.1.0 → 2.0.1`, `FINDINGS.md`, and checkout-only `verify.py`.
- Decision: graduate targeted `vendor/clipper2-rust` port (with priced wall ceiling) OR close if t169 already absorbs `f2`.
- Map: destination-status block and Decisions-so-far line.

## Answer

**Resolved 2026-11-20: the ticket's premise is falsified. The remaining
`~2.1x–2.5x` gap is not a Clipper2 1.1.0 → 2.0.1 kernel-version gap; it is one
missing Orca fork patch, `clipper.offset.cpp::OffsetConcaveCrossing` (~30
lines). Graduate [t172](172-port-offset-concave-crossing.md); no production edit
was made by this screen.**

**The decisive measurement.** Applying that patch to bare Clipper2 1.5.2 makes
its output **byte-identical to canonical 2.0.1 on 87/87** IB and `0.04 mm Miter 3`
opening cases (`IB_CAND_POS`, `IB_DEEP_NEG`, `IB_DEEP_POS`, `SH_OPEN_TOP_CANON`,
`SH_OPEN_BOT_CANON`). Without it, 2.0.1 is **0.78x / 0.87x** of 1.5.2 on the two
canonical openings, and the patch is worth **2.03x (BOT) / 2.33x (TOP)** on
1.5.2 and **2.28x (BOT) / 1.70x (TOP)** on 2.0.1. The `f2` factor is the patch, not the version.
`DoRound`'s `std::round`-vs-`ceil` is the second Orca patch, visible only on the
`0.20 mm Round` legacy openings (8/31 empty cases match; the rest differ by the
Round join's step count).

**The Rust port.** `vendor/clipper2-rust` is crate 1.1.0 but a **1.5.4** C++
port, and it lacks the patch. Porting it into a scratch copy captures **81–97 %**
of the remaining per-call gap and brings the hot sites to **1.04–1.30x** of
canonical:

| corpus / scope | rust before | rust + patch | canonical | gain | after / canonical |
| --- | ---: | ---: | ---: | ---: | ---: |
| prepass IB offsets (56) | 1,326.3 ms | 711.4 ms | 631.5 ms | **1.86x** | 1.13x |
| prepass `SH_OPEN_*` (62) | 751.4 ms | 459.6 ms | 439.9 ms | **1.63x** | 1.05x |
| `t68sites` base offsets (123) | 713.7 ms | 225.2 ms | 187.9 ms | **3.17x** | 1.20x |
| `t68sites` benchy offsets (54) | 116.1 ms | 45.4 ms | 40.9 ms | **2.56x** | 1.11x |

Per-site the classic-perimeter offsets go **2.95–4.57x → 1.07–1.27x**, the two
canonical openings **2.38–2.70x → 1.04–1.30x**, and the vertex totals converge on
canonical (`A0` 1.082 → 1.007, `A1` 1.073 → 0.998). Booleans are untouched
(`D/H/J` 0.61–0.94x, already faster than canonical). The scratch crate passes
**403/403** of its own tests; the patch is
[`offset_concave_crossing.patch`](../evidence/t170-clipper2-201-screen/offset_concave_crossing.patch).

**Price on the post-t169 tree** (two-step estimate: t169's self-time→wall
derivative × the corpus patch gain): central **+16.4 %** on `base classic-off`
(required 3.9 %) and **+15.3 %** on `base arachne-off` (required 10.3 %); **5.2–11.4 %**
on the other six. The conservative host-only estimate clears the 0.80 % floor in
**8/8** (5.2x–16.3x). **But the kernel route cannot be the whole route:** even at
zero cost for every host and guest offset, six cells stay short of their cut
(`base classic-on` 23.3 % vs 52.7 %; `base arachne-on` 21.6 % vs 50.8 %; the
benchy cells 10.7–26.1 % vs 39.0–58.5 %). The two base supports-off cells are the
only ones any offset-kernel work can win.

**The sub-factors.** (a) Flat-path pass-1 `offset2_ex`: **not a factor** — every
driver already runs pass 1 flat, and Arm 1 was banked on 2026-07-11. (b)
`PreserveCollinear(false)`: **not a factor on offsets** — both offset
constructors pass `false`; Rust's boolean path keeps the default `true`, but
booleans measure 0.61–0.94x (Rust faster). (c) 1.5.2 → 2.0.1: **zero with the
patch, negative without it**. (d) `c2_offset_expolygons` grouped mode:
**exercised on all 118 prepass cases** (`stray_holes = 0`); the per-path fallback
fires on 6/170 `t68sites` cases only, and those sites still close to 1.07–1.27x.

**Upstream.** `clipper2-rust` 1.2.0 (2026-09-18) is still a 1.5.4 port and does
not contain the patch; there is no 2.x release. A targeted vendor patch is the
cheapest exact path.

**What is not claimed.** The whole-cell numbers are an estimate, not a paired
campaign; the patch moves per-layer geometry and needs a **t81-style exception**
(the sixth, if granted) and a fresh reference set; the Rust patched output is
**not byte-identical** to canonical (the 1.5.4 port's rounding differs), only
converged; and the guest-fuel ceiling applies the host derivative to guest CPU,
so it is an upper bound. Evidence:
[`evidence/t170-clipper2-201-screen/`](../evidence/t170-clipper2-201-screen/FINDINGS.md)
(`verify.py` PASS, `--self-test` 6/6 negative controls). **Frontier:** t172
(the port, HITL ruling first), then
[171](171-post-t169-prepass-reprice.md) (still open, unclaimed).
