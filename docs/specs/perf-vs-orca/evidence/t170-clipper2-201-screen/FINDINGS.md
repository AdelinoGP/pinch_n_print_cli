# t170 findings: Clipper2 1.1.0 → 2.0.1 "kernel upgrade" screen

**Verdict (2026-11-20): the premise is corrected — the remaining offset gap is
not a Clipper2 kernel-version gap, it is one missing Orca fork patch.** Porting
`clipper.offset.cpp::OffsetConcaveCrossing` (≈30 lines) into the vendored Rust
port captures **81–97 %** of the remaining per-call gap on every hot offset site
and **89–94 %** in aggregate, and when the same patch is applied to C++ 1.5.2 it
reproduces canonical 2.0.1 **byte-for-byte on all 87** IB / canonical-opening
cases. Without the patch, 2.0.1 is **slower** than 1.5.2 (`0.78x` / `0.87x`) on
those openings; the kernel upgrade contributes **nothing**. The cheapest exact
port path is the ~30-line Rust patch, not a kernel replacement. It graduates
[t172](../../issues/172-port-offset-concave-crossing.md); no production edit is
made by this screen.

The patch is also the first lever in this map that **wins a cell on paper**:
it is estimated at **+16.4 %** wall on `base classic-off` against a **3.9 %**
required cut, and **+15.3 %** on `base arachne-off` against **10.3 %**. It clears
the 0.80 % candidate floor in all eight cells (smallest conservative estimate
**4.2 %**, 5.2x the floor). But the same arithmetic shows the kernel route
**cannot** be the whole route: even at **zero cost** for every host and guest
offset, the other six cells remain short of their required cut (strongest
`base classic-on` 23.3 % vs 52.7 %).

Ticket: [170](../../issues/170-clipper2-2-0-1-kernel-port-screen.md). All numbers
below are re-derived by [`verify.py`](verify.py) from the
[`reduced/`](reduced/) set; `--self-test` rejects **6/6** negative controls.

## 1. What was measured

The t167 replay harness was extended with five same-window C++ builds and three
Rust builds. All engines run the same 118-case `base-prepass-t167.corpus` and
the 170-case / 75-case `t68sites` corpora; each case is warmed once and timed
five times forward plus five times reverse, median per case.

| engine | kernel | Orca `OffsetConcaveCrossing` | decimation |
| --- | --- | --- | --- |
| `cpp_152` | Clipper2 1.5.2 | no | no |
| `cpp_152_concave` | Clipper2 1.5.2 | **yes** | no |
| `v250_nodecim` | Clipper2 2.0.1 (Orca fork) | yes | no |
| `v250_nodecim_noX` | Clipper2 2.0.1 | **no** | no |
| `v250_decim` | Clipper2 2.0.1 | yes | yes |
| `v250_decim_noX` | Clipper2 2.0.1 | **no** | yes |
| `rust_base` | `vendor/clipper2-rust` 1.1.0 (1.5.4 port) | no | **yes** (t169 harvest) |
| `rust_patched` | same + ported patch | **yes** | **yes** |

Compiler: `g++ (MinGW-Builds) 16.1.0`, `-O3 -std=c++17 -static`; Rust:
`cargo --release --offline`, `mimalloc` global allocator. C++ drivers link the
t167 `oneapi/tbb` shim (scalable allocator → `std::malloc`), so canonical's
production allocator is *stronger* than the measured one; any bias favours the
Rust arm and makes the measured gap conservative.

## 2. The decisive result: 1.5.2 + the Orca patch == 2.0.1

On every IB case and every `0.04 mm Miter 3` shell opening, the patched 1.5.2
binary's output hash equals the canonical 2.0.1 hash:

| site | n | `cpp_152 + patch` == `v250` | `cpp_152` == `v250` |
| --- | ---: | ---: | ---: |
| `IB_CAND_POS` | 16 | **16/16** | 0/16 |
| `IB_DEEP_NEG` | 16 | **16/16** | 2/16 |
| `IB_DEEP_POS` | 24 | **24/24** | 0/24 |
| `SH_OPEN_BOT_CANON` | 15 | **15/15** | 1/15 |
| `SH_OPEN_TOP_CANON` | 16 | **16/16** | 0/16 |
| **total** | **87** | **87/87** | 3/87 |

The `0.20 mm Round` legacy openings match only on their empty cases (8/31),
because canonical also carries a second Orca patch — `DoRound` rounds the step
count (`std::round`) where 1.5.2/1.5.4 use `ceil`. That is an output-parity item,
not a speed lever of the same size.

The 3-way ablation, re-derived in this window (t167's numbers in parentheses):

| site | `f2` total (1.5.2 → 2.0.1 nodecim) | `f2` **without** the patch | patch factor on 2.0.1 | patch factor on 1.5.2 |
| --- | ---: | ---: | ---: | ---: |
| `SH_OPEN_TOP_CANON` | 1.48 (1.76) | **0.87** | 1.70 | 2.33 |
| `SH_OPEN_BOT_CANON` | 1.78 (1.72) | **0.78** | 2.28 | 2.03 |
| `SH_OPEN_TOP_LEGACY` | 0.87 (0.98) | **0.64** | 1.37 | 1.67 |
| `SH_OPEN_BOT_LEGACY` | 0.86 (0.92) | **0.54** | 1.60 | 1.86 |
| `IB_DEEP_NEG` | 1.19 (1.23) | 0.96 | 1.24 | **9.92** |
| `IB_DEEP_POS` | 1.05 (1.13) | 0.86 | 1.21 | **4.02** |

Read the middle column: with the patch removed, canonical 2.5.0's 2.0.1 kernel
is **13–46 % slower** than Clipper2 1.5.2 on the very sites the ticket was
written about. The `f2` factor is the patch, not the version. The 1.5.2 kernel's
3-point concave-loop cleanup is pathological on dense inward offsets, which is
why the same patch is worth **9.9x** there and only 1.2x on 2.0.1 — but 2.0.1's
gain is still 1.7–2.3x on the canonical openings.

**Window disclosure.** t167 measured `f2` at 1.72/1.76; this window re-derives
1.78/1.48. The per-window swing is up to 16 % on a single site, so the *product*
is not a precise constant. The identity result is window-independent and is the
part the decision rests on.

## 3. The Rust port with the patch

`vendor/clipper2-rust` is crate version 1.1.0 but a **1.5.4** C++ port
(`src/version.rs`), so the real residual is 1.5.4 → Orca's 2.0.1 fork. With the
t169 decimation applied to both Rust arms, the patch moves every hot site:

| corpus / scope | n | rust base | rust + patch | canonical 2.0.1 | base / patched | patched / canonical |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| prepass offsets (`IB_*`) | 56 | 1,326.3 ms | **711.4 ms** | 631.5 ms | **1.86x** | 1.13x |
| prepass two-pass (`SH_OPEN_*`) | 62 | 751.4 ms | **459.6 ms** | 439.9 ms | **1.63x** | 1.05x |
| `t68sites` base offsets (`A*..K`) | 123 | 713.7 ms | **225.2 ms** | 187.9 ms | **3.17x** | 1.20x |
| `t68sites` benchy offsets | 54 | 116.1 ms | **45.4 ms** | 40.9 ms | **2.56x** | 1.11x |

Per-site, on the canonical `0.04 mm Miter 3` openings:

| site | rust / canonical before | after | patch gain |
| --- | ---: | ---: | ---: |
| `SH_OPEN_TOP_CANON` | 2.50–2.64x | **1.04–1.23x** | 2.0–2.5x |
| `SH_OPEN_BOT_CANON` | 2.38–2.70x | **0.98–1.30x** | 2.1–2.4x |
| `IB_DEEP_NEG` | 2.57–2.59x | **1.28–1.30x** | 2.0x |
| `IB_DEEP_POS` | 1.94–2.06x | **1.06–1.11x** | 1.85x |
| `A1` (classic perimeter offset) | 4.42x | **1.27x** | 3.47x |
| `E` | 4.57x | **1.27x** | 3.60x |
| `K` | 4.32x | **1.21x** | 3.58x |
| `B` / `F` | 3.82 / 4.09x | **1.14 / 1.15x** | 3.3–3.6x |
| `A0` / `C` / `G` | 2.95 / 3.00 / 3.05x | **1.21 / 1.07 / 1.07x** | 2.4–2.8x |

Booleans are untouched, as expected: `D` 0.92→0.91, `H` 0.94→0.93, `J` 0.63→0.61
(Rust already faster than canonical). The patch removes no features: empty-case
counts are identical before and after on every site, and vertex totals converge
on canonical — e.g. `A0` 1.082 → **1.007** of canonical, `A1` 1.073 → **0.998**,
`E` 1.061 → **0.998**, `K` 1.068 → **0.997**. Median relative area vs canonical
is ≤ 0.007 % on the offset sites.

The patched scratch crate passes its own suite: **403 passed, 0 failed**
(`cargo test --release`). The patch is
[`offset_concave_crossing.patch`](offset_concave_crossing.patch).

## 4. Why `f2` was misread as a kernel gap

The 2.0.1 `clipper.offset.cpp` is **686** lines against 1.5.2's **658**, and the
delta is mostly Orca's own patches, not upstream:

- **`OffsetConcaveCrossing`** (Orca): joins concave corners at the crossing of
  both edge offsets when safe, instead of emitting three points that the final
  union must remove. This is the measured lever.
- **`DoRound` step count** (Orca): `std::round(...)` instead of `ceil(...)`, so
  round joins keep canonical vertices. Missing in the Rust port; output-parity
  only.
- **`GetLowestClosedPathInfo`** (2.0.1): already in the Rust port.
- **`preserve_collinear_` plumbing** into the offset cleanup (2.0.1): already in
  the Rust port; both sides default `false` on the offset path.
- **tbbmalloc node allocators** (Orca): not applicable — the Rust engine uses
  arena `Vec` indices, not per-node allocation, and ships mimalloc.
- **`GetLineIntersectPt`** (2.0.1): still missing in Rust. It is a
  robustness/rounding change on the boolean engine; booleans already measure
  0.61–0.94x (Rust faster), so it is not a speed lever here.

The 2.0.1 `clipper.engine.cpp` is 3,189 lines against the Rust port's 3,357; the
Rust port already carries `Path2ContainsPath1`, the adjacent micro
self-intersection repair (`duplicate_op`), the `CheckSplitOwner` recursion and
the `TopX` joined-edge change. Nothing in that diff is a measured gap.

**Upstream availability.** `clipper2-rust` 1.2.0 (crates.io, 2026-09-18) is still
a **1.5.4** port (`src/version.rs`); its release adds `using_z` and a
`clean_collinear` fix, and it does **not** contain `OffsetConcaveCrossing`.
There is no 2.x release. A kernel-version bump is therefore neither available
nor useful; the targeted patch is the route.

## 5. The ticket's four sub-factors

| sub-factor | verdict |
| --- | --- |
| **(a) flat-path pass-1 `offset2_ex`** | **Not a factor in this corpus.** The 1.5.2 driver, the canonical 2.0.1 driver and the Rust driver all run pass 1 into flat `Paths64`; t169 established that PnP's Arm 1 landed on 2026-07-11. There is no intermediate `PolyTree64` on this path to remove. |
| **(b) `PreserveCollinear(false)`** | **Not a factor on offsets.** Canonical sets it on the *boolean cleanup* (`c2_clip`) and the 2.0.1 fork plumbs it into the offset cleanup with a default of `false`; the 1.5.2 and Rust offset constructors also pass `false`. On the boolean path Rust keeps the default `true`; booleans still measure 0.61–0.94x (Rust faster), so it is a parity residual, not a speed gap. |
| **(c) Clipper2 1.5.2 → 2.0.1** | **Zero on the measured sites once the Orca patch is present** (87/87 byte-identical); **negative without it** (0.78–0.87x). The version is not the lever. |
| **(d) `c2_offset_expolygons` single-group** | **Grouped on all 118 prepass cases.** Canonical `c2_offset_as_group` returns true throughout: `stray_holes = 0` on every case. The per-path + final-union fallback fires only on **6 of 170** `t68sites` cases (7 stray holes); the patch still closes those sites to 1.07–1.27x, so the fallback is a disclosed residual, not the driver. |

## 6. Whole-cell price on the post-t169 tree

The price combines two measured quantities, and is an **estimate**, not a
scoreboard:

1. **Self-time → wall calibration** from t169's own paired removal:
   `k_cell = t169 wall-saving % / (host offset + offset2_ex self-seconds removed)`.
   This is a local derivative at the pre-t169 point; t169 removed decimation
   work, while this patch removes concave-loop/cleanup work, so `k` is an
   approximation.
2. **Patch gain** from the corpus: `offset` scope **1.86–3.17x** (central
   2.45x), `offset2_ex` scope **1.50–2.00x** (central 1.635x, the measured
   aggregate). Self-time saved is `S × (1 − 1/gain)`.

Central host-only estimate, conservative→optimistic band, and the two ceilings:

| cell | `k` (%/s) | required cut | patch price (host) | zero-cost ceiling host / host+guest |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.220 | 39.0 % | 6.0 / **7.6** / 9.0 % | 14.1 / 22.3 % |
| benchy classic-on | 2.297 | 57.9 % | 9.0 / **11.4** / 13.3 % | 20.1 / 26.1 % |
| benchy arachne-off | 2.397 | 45.3 % | 4.2 / **5.2** / 6.2 % | 9.9 / 10.7 % |
| benchy arachne-on | 2.265 | 58.5 % | 8.8 / **11.1** / 13.0 % | 19.8 / 20.7 % |
| **base classic-off** | 0.568 | **3.9 %** | 13.1 / **16.4** / 19.3 % | 30.2 / 44.7 % |
| base classic-on | 0.156 | 52.7 % | 8.6 / **10.9** / 12.8 % | 19.2 / 23.3 % |
| **base arachne-off** | 0.540 | **10.3 %** | 12.2 / **15.3** / 18.1 % | 28.4 / 29.7 % |
| base arachne-on | 0.164 | 50.8 % | 9.5 / **12.0** / 14.0 % | 21.0 / 21.6 % |

The `required cut` column is t169's §9 estimate against t163's board, not a
fresh two-tool measurement; the **verdict is the ratio of two estimates**, so
treat "wins" as "the next ticket must measure this cell first", not as a score.

Two readings bind the route:

- **The patch clears the 0.80 % candidate floor in 8/8**, by 5.2x (benchy
  arachne-off) to 16.3x (base classic-off), on the conservative host-only
  estimate. This is enough to graduate.
- **No offset-kernel work can win six of the eight cells.** Even if every host
  `offset` / `offset2_ex` self-second and every guest offset CPU-second became
  free, the ceiling column stays below the required cut for every cell except
  the two base supports-off cells. `base classic-on` needs 52.7 % and tops out
  at 23.3 %; `base arachne-on` needs 50.8 % and tops out at 21.6 %; the four
  benchy cells top out at 10.7–26.1 % against 39.0–58.5 %. The route needs a
  second phase for those six.

This also explains why t167's "4.96x slower on offsets" looked like an engine
rewrite: the 4.96x was mostly the missing patch, and the patch is 30 lines.

## 7. What this does and does not claim

- It **does** claim that the concave-crossing patch is the f2 factor on the
  measured Miter offsets/openings, at 87/87 byte identity with canonical on
  Clipper2 1.5.2.
- It **does** claim the Rust port's residual after the patch is 1.04–1.30x on
  the hot sites, so the patch captures 81–97 % of the per-call gap and 89–94 %
  in aggregate.
- It does **not** claim a measured whole-cell wall: the §6 price is a
  two-step estimate, and no paired PnP A/B was run. The follow-up ticket must
  run the standing gate.
- It does **not** claim the patch is output-neutral. It moves per-layer and
  prepass geometry (it removes the 3-point concave loops canonical does not
  emit), so it needs a **t81-style exception** and a fresh reference set. The
  human granted the fifth exception for t169's decimation; this would be the
  sixth, and it is not assumed.
- The Rust patched output is **not byte-identical** to canonical (the 1.5.4
  port's rounding differs); vertex counts converge and area deltas are tiny,
  but this is a parity-direction change, not a byte-identical refactor.
- The guest-fuel term in the `host+guest` ceiling applies the host `k` to guest
  CPU seconds. Guest work may overlap differently; that column is an **upper
  bound**, which is why the six-cell "unreachable" verdict uses it as the
  strongest form.

## 8. Reproduce

The C++ engines are rebuilt from the two checkouts and the scratch sources in
`.local-artifacts/perf-vs-orca/t170-clipper2-screen/` (not committed):

```bash
# C++ f2 ablation (7 engines, 118 cases, forward+reverse)
python ablate.py
# Rust patch screen on the three corpora (base harness = vendored tree,
# patched harness = the scratch copy with offset_concave_crossing.patch)
python ablate_rust.py
python ablate_rust.py <t69>/corpus/t68sites/base-t68sites.corpus   ablation_rust_t68base.json
python ablate_rust.py <t69>/corpus/t68sites/benchy-t68sites.corpus ablation_rust_t68benchy.json
python price.py
python reduce_t170.py
```

Checkout-only verification:

```bash
python verify.py --self-test   # 6/6 negative controls rejected
python verify.py               # 0 failures
```

The patch's own crate passes `cargo test --release` (403/403) in the scratch
copy; the in-tree vendor is untouched by this ticket.
