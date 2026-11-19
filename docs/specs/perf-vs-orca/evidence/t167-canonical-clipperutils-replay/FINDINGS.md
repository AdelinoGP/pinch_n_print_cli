# Ticket 167 Findings: Canonical ClipperUtils Replay on PnP's Prepass & Core Operands

**Verdict (2026-11-19): Branch A fires — canonical is ≥ 3x faster on the hot offset sites (`canonical_ge_3x_faster_on_hot_offset_sites`).**

Replaying 413 captured cases (295 from `t69` + 118 newly captured `ShellClassification` Pass 1 `apply_opening` and `gate_internal_bridge_sites` operands from `base classic-off`) across four engines falsifies `t70`'s "kernel at parity" reading on **offsets** while confirming it on **booleans**, and corrects a major premise about what canonical actually runs at the `v2.5.0` pin.

---

## 1. Major Premise Correction at the Canonical `v2.5.0` Pin

Ticket 167's question was written against the older `OrcaSlicerDocumented` checkout (`3e1daccd7c`, 2026-09-15), where `ClipperUtils.cpp` wrapped **Clipper1** (`ClipperLib 6.2.6`).

At the map's pinned canonical checkout (`D:\slicerProject\Orca(pnp_gui)`, `v2.5.0`, commit `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`, 2026-11-07), **OrcaSlicer already migrated `ClipperUtils.cpp` completely off Clipper1 onto Clipper2 2.0.1** in commit `222c6a2df5` (*"Improve performance by migrating to Clipper2 2.0.1 (#15969)"*, 2026-10-02, deleting `deps_src/clipper/`).

To answer both the pin-accurate canonical question and the literal Clipper1 question without ambiguity, this replay measures **four** engines on identical operands and parameters:

| Engine ID | Kernel & Wrapper | Coordinates | Allocator (disclosed) |
|---|---|---|---|
| `rust_pnp` | In-tree `vendor/clipper2-rust` 1.1.0 (t70-optimized) + PnP `polygon_ops` wrapper (`offset`, `offset2_ex`, `difference`/`union`/`intersection`) | 100 nm (`i64`) | `mimalloc::MiMalloc` (matches `pnp_cli`) |
| `cpp_c2_1_5_2` | Bare C++ Clipper2 1.5.2 (`OrcaSlicerDocumented/deps_src/clipper2`, t69 continuity twin + opcode 5) | 100 nm (`int64_t`) | MinGW `g++` CRT `malloc` |
| `orca_v2_5_0` | **Canonical `v2.5.0` (`40ecc66f78`)**: Clipper2 2.0.1 + verbatim `ClipperUtils.cpp` (`append_offset_path` edge decimation `0.005 * \|delta\|`, `c2_offset_expolygons`/`c2_offset_closed`, flat-path pass-1 `offset2_ex`, `PreserveCollinear(false)` booleans, `c2_to_expolygons` with stray-hole attachment) | 1 nm (×100 outside timer) | MinGW `g++` CRT `malloc` (`tbb::scalable_allocator` shimmed to `std::malloc`) |
| `orca_clipper1` | Pre-2.5.0 Orca (`3e1daccd7c`): Clipper1 (`ClipperLib 6.2.6`) + verbatim `ClipperUtils.cpp` (`ShortestEdgeLength = 0.005 * \|delta\|`, `offset_expolygon_inner`, `offset_paths`, `offset2_ex`) | 1 nm (×100 outside timer) | MinGW `g++` CRT `malloc` (`tbb::scalable_allocator` shimmed to `std::allocator`) |

**Allocator disclosure:** `rust_pnp` runs with `mimalloc` (matching production `pnp_cli`), whereas all three C++ drivers link MinGW CRT `malloc` via `shims/oneapi/tbb/scalable_allocator.h` rather than Orca's native `tbbmalloc`. Any allocator bias favors `rust_pnp`; the measured C++ speedups are **conservative lower bounds** relative to Orca's production `tbbmalloc` build.

**Geometric control verification:** Before the campaign, `verify_drivers.py` verified all four engines against 8 synthetic controls (square grow/shrink, hole grow/shrink, miter/round openings, boolean diff/intersection, empty collapse). All four engines agreed to ≤ 1e-6 relative area on all 7 miter/boolean/empty controls and 0.00003% on round opening.

---

## 2. Provenance & Prepass Capture Census

- **PnP commit:** `8220a901e3510532a1a9dbabe6ebe5562eadaa68` (`crates/` clean — temporary capture hook reverted immediately after capture and preserved only as `probe-t167-dump.patch`).
- **Canonical `v2.5.0` commit:** `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19` (Clipper2 `2.0.1`, migration `222c6a2df5`).
- **Pre-2.5.0 checkout commit:** `3e1daccd7c567a0a3d2b5721841d30845f21307e` (Clipper1 `6.2.6` + Clipper2 `1.5.2`).
- **Compilers:** `rustc 1.96.0 (ac68faa20 2026-05-25)` (`--release --offline`); `g++ (MinGW-Builds) 16.1.0` (`-O3 -std=c++17 -static -static-libgcc -static-libstdc++`).
- **Capture neutrality:** The capture slice of `tmp/base.stl` (`pnp-classic-supports-off.json`) produced G-code SHA-256 `0a1ee3df67e5e9713f8acf8428fb44d2a8a3bde2b236de5bdf8ecb03b903af9e`, **byte-identical** to `t166`'s active `base classic-off` reference.

Full prepass census (1,339 calls on `base classic-off`) vs deterministic quantile-stratified replay sample (`corpus/base-prepass-t167.corpus`, 118 cases, 8.43 MB):

| Site | Op | Parameters | Full Census Calls | Full `in_verts` med / max | Sample Cases | Sample `in_verts` med / max |
|---|---|---|---|---|---|---|
| `SH_OPEN_TOP_CANON` | `opening_ex` (5) | `0.04 mm`, Miter 3.0 | 487 | 5,866 / 10,165 | 16 | 5,749 / 8,615 |
| `SH_OPEN_TOP_LEGACY` | `opening_ex` (5) | `0.20 mm`, Round | (same 487 layers) | 5,866 / 10,165 | 16 | 5,749 / 8,615 |
| `SH_OPEN_BOT_CANON` | `opening_ex` (5) | `0.04 mm`, Miter 3.0 | 487 | 4,917 / 11,678 | 15 | 4,864 / 11,117 |
| `SH_OPEN_BOT_LEGACY` | `opening_ex` (5) | `0.20 mm`, Round | (same 487 layers) | 4,917 / 11,678 | 15 | 4,864 / 11,117 |
| `IB_DEEP_POS` | `offset` (0) | `+0.5356 mm`, Miter 2.0 | 122 | 8,270 / 14,536 | 24 | 8,172 / 14,536 |
| `IB_DEEP_NEG` | `offset` (0) | `-1.6069 mm`, Miter 2.0 | 51 | 1,046 / 4,610 | 16 | 1,029 / 4,610 |
| `IB_CAND_POS` | `offset` (0) | `+0.3571 mm`, Miter 2.0 | 192 | 24 / 179 | 16 | 24 / 179 |

---

## 3. Single-Thread Replay Results (Fresh Process per Case, 1 Warmup + 5 Repeats, Forward & Reverse)

Single-thread pass wall times: `single/forward` 147.0 s, `single/reverse` 145.3 s (413 cases; 405 unique excluding `t55/accelerated-*` mirrors). Forward/reverse output hashes and geometries matched 100% across all 4 engines and all 413 cases.

### 3.1 By Opcode Class (405 unique cases)

| Opcode | n | `rust_pnp` sum (ms) | `cpp_c2_1_5_2` sum (ms) | `orca_v2_5_0` sum (ms) | `orca_clipper1` sum (ms) | `rust / orca_v2_5_0` (fwd / rev) | `rust / cpp_1_5_2` (fwd / rev) | `rust / orca_c1` (fwd / rev) | `orca_v2_5_0` med / max Δarea | `orca_v2_5_0` med verts ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **`offset`** | 233 | 4,561.8 | 3,676.2 | **920.0** | 1,177.9 | **4.96x / 5.02x** | 1.24x / 1.25x | 3.87x / 3.91x | 0.0032% / 49.21% | 0.938 |
| **`opening_ex`** | 62 | 1,223.8 | 944.9 | **490.7** | 268.2 | **2.49x / 2.55x** | 1.30x / 1.29x | 4.56x / 4.63x | 0.0365% / 0.370% | 0.965 |
| `difference` | 68 | 44.8 | 53.7 | 56.4 | 40.9 | **0.79x / 0.78x** | 0.83x / 0.80x | 1.10x / 1.05x | 0.0328% / 1.333% | 1.124 |
| `intersection` | 34 | 280.4 | 221.8 | 225.3 | 132.1 | **1.24x / 1.24x** | 1.26x / 1.24x | 2.12x / 2.08x | 0.0022% / 8.937% | 1.000 |
| `union` | 8 | 761.3 | 546.6 | 547.4 | 692.4 | **1.39x / 1.41x** | 1.39x / 1.43x | 1.10x / 1.05x | 0.0000% / 0.000% | 1.000 |
| **All unique** | **405** | **6,872.0** | **5,443.2** | **2,239.9** | **2,311.6** | **3.07x / 3.12x** | **1.26x / 1.27x** | **2.97x / 2.94x** | **0.0044% / 49.21%** | **0.958** |

**Key structural split:**
- **Booleans are at parity (`0.79x–1.41x`)**: `cpp_c2_1_5_2 / orca_v2_5_0` is **0.95x–1.00x** across `difference`, `intersection`, and `union`. `t70`'s parity claim holds for boolean clips.
- **Offsets are 4.96x–5.02x faster in canonical `orca_v2_5_0`** (`3.83x` median per-case ratio), and **`orca_v2_5_0` beats `t69`'s own `cpp_c2_1_5_2` twin by 4.00x** (`3,676.2 → 920.0 ms`). Even on `t69`'s existing `base-t68sites.corpus` (`n = 170`), `rust_pnp / orca_v2_5_0` is **3.92x / 3.97x** (`benchy-t68sites.corpus`: **3.11x / 3.13x**). That explains why `t69`/`t70` missed the gap: `t69` compared Rust Clipper2 1.1.0 against *bare* C++ Clipper2 1.5.2 without Orca's `ClipperUtils` wrapper or Clipper2 2.0.1.

### 3.2 By Site (Hot Prepass Sites + `t69` Sites)

| Site | n | `in_verts` med | `rust` med (ms) | `c152` med (ms) | `v250` med (ms) | `c1` med (ms) | `rust / v250` sum (fwd / rev) | `rust / c1` sum (fwd / rev) | `v250` med / max Δarea |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **`IB_DEEP_NEG`** | 16 | 1,029 | 13.38 | 10.46 | **1.16** | 0.84 | **12.76x / 12.28x** | 15.48x / 14.80x | 6.443% / 49.21% |
| **`IB_DEEP_POS`** | 24 | 8,172 | 132.62 | 110.90 | **24.32** | 18.85 | **4.56x / 4.64x** | 5.43x / 5.56x | 0.349% / 2.026% |
| **`SH_OPEN_TOP_CANON`** | 16 | 5,749 | 8.37 | 7.39 | **3.39** | 3.82 | **2.45x / 2.57x** | 2.02x / 2.04x | 0.117% / 0.370% |
| **`SH_OPEN_BOT_CANON`** | 15 | 4,864 | 7.98 | 7.01 | **2.69** | 2.97 | **2.45x / 2.56x** | 2.02x / 2.06x | 0.097% / 0.230% |
| **`SH_OPEN_TOP_LEGACY`** | 16 | 5,749 | 32.06 | 24.12 | **9.54** | 5.72 | **2.57x / 2.59x** | 6.65x / 6.69x | 0.004% / 0.036% |
| **`SH_OPEN_BOT_LEGACY`** | 15 | 4,864 | 26.68 | 21.43 | **8.79** | 4.23 | **2.43x / 2.49x** | 6.45x / 6.71x | 0.005% / 0.080% |
| `IB_CAND_POS` | 16 | 24 | 0.01 | 0.01 | 0.01 | 0.01 | 1.61x / 1.69x | 0.95x / 0.93x | 0.003% / 4.463% |
| `A1` (`t68sites` offset) | 23 | 2,418 | 12.30 | 9.14 | **1.59** | 4.62 | **5.79x / 6.08x** | 2.29x / 2.32x | 0.013% / 1.280% |
| `E` (`t68sites` offset) | 23 | 1,770 | 6.72 | 4.84 | **1.38** | 3.59 | **5.01x / 5.07x** | 1.82x / 1.78x | 0.003% / 0.070% |
| `A0` (`t68sites` offset) | 23 | 3,052 | 8.89 | 6.54 | **2.22** | 4.24 | **4.33x / 4.26x** | 2.09x / 2.06x | 0.005% / 0.070% |
| `B` (`t68sites` offset) | 22 | 2,695 | 6.28 | 5.22 | **1.51** | 3.50 | **4.19x / 4.26x** | 1.76x / 1.76x | 0.001% / 0.040% |
| `K` (`t68sites` offset) | 23 | 1,770 | 4.74 | 3.49 | **1.05** | 2.65 | **4.12x / 4.26x** | 1.60x / 1.63x | 0.002% / 0.030% |
| `F` (`t68sites` offset) | 23 | 1,770 | 3.25 | 2.58 | **0.85** | 2.10 | **3.79x / 3.77x** | 1.45x / 1.44x | 0.001% / 0.130% |
| `G` (`t68sites` offset) | 18 | 1,980 | 2.38 | 2.23 | **0.84** | 1.78 | **3.07x / 3.03x** | 1.40x / 1.40x | 0.003% / 0.010% |
| `C` (`t68sites` offset) | 22 | 1,953 | 2.70 | 2.45 | **0.90** | 2.09 | **3.02x / 3.06x** | 1.35x / 1.39x | 0.002% / 0.240% |
| `D`, `H`, `J` (`difference`) | 68 | 1,612 | 0.27 | 0.32 | 0.37 | 0.25 | **0.64–0.80x** | 0.71–1.13x | 0.033% / 1.333% |
| `RP1`, `RP2` (`intersection`) | 34 | 6,576 | 1.44 | 1.14 | 1.16 | 0.94 | **1.20–1.29x** | 2.06–2.17x | 0.002% / 8.937% |
| `550` (`t55` union) | 8 | 29,660 | 59.35 | 40.07 | 40.22 | 49.30 | **1.39x / 1.41x** | 1.10x / 1.05x | 0.000% / 0.000% |

Every single offset site in both the new prepass corpus (`IB_DEEP_NEG`, `IB_DEEP_POS`) and `t69`'s existing `t68sites` (`A0`, `A1`, `B`, `C`, `E`, `F`, `G`, `K`) is **3.02x–12.76x faster** in canonical `orca_v2_5_0`!

---

## 4. 12-Way Concurrent Loaded Replay (Memory-Bandwidth & Cache-Pressure Test)

Twelve simultaneous worker processes each ran the full 320-case base-model corpus (`base-prepass-t167` + `base-t68sites` + `base-region-prep` + `ordinary-base`) in cyclically staggered order (`forward` and `reverse`). Every output hash matched single-thread 100%.

### 4.1 Whole-Batch 12-Worker Wall Time (320 base cases × 12 workers)

| Engine | Forward wall (s) | Worker median (s) | Reverse wall (s) | Worker median (s) | Ratio vs `orca_v2_5_0` (fwd / rev) |
|---|---:|---:|---:|---:|---:|
| `rust_pnp` | 77.13 | 73.93 | 64.36 | 62.79 | **3.34x / 2.96x** |
| `cpp_c2_1_5_2` | 54.23 | 51.09 | 49.02 | 47.49 | **2.35x / 2.25x** |
| **`orca_v2_5_0`** | **23.09** | **21.07** | **21.75** | **20.23** | **1.00x / 1.00x** |
| `orca_clipper1` | 21.86 | 20.16 | 21.59 | 19.96 | 0.95x / 0.99x |

*(Disclosure: the `loaded12/forward` `rust_pnp` pass began at 57% host CPU load while the other 7 loaded passes began at 1–12% load; `loaded12/reverse` `rust_pnp` began at 1% load and took 64.36 s vs 21.75 s for `orca_v2_5_0` = **2.96x** whole-corpus wall even with the parity-bound `t55` unions and `region-prep` intersections included.)*

### 4.2 Per-Opcode & Hot-Site Loaded Ratios and Loaded/Single Slowdown Factors

| Group / Site (base subset) | n | Loaded12 `rust / v250` (fwd / rev) | Loaded12 `rust / c1` (fwd / rev) | Slowdown `loaded12 / single` `rust_pnp` (fwd / rev) | Slowdown `loaded12 / single` `orca_v2_5_0` (fwd / rev) |
|---|---:|---:|---:|---:|---:|
| **`opcode/offset`** | 179 | **5.78x / 5.16x** | 4.54x / 4.04x | 1.65x / 1.46x | 1.43x / 1.44x |
| **`opcode/opening_ex`** | 62 | **2.93x / 2.70x** | 6.28x / 5.74x | 1.71x / 1.62x | 1.46x / 1.54x |
| `opcode/difference` | 47 | 1.11x / 1.03x | 1.64x / 1.53x | 1.67x / 1.61x | 1.18x / 1.21x |
| `opcode/intersection` | 28 | 1.39x / 1.30x | 2.29x / 2.15x | 1.63x / 1.61x | 1.44x / 1.52x |
| `opcode/union` | 4 | 1.49x / 1.38x | 1.42x / 1.31x | 1.81x / 1.65x | 1.68x / 1.68x |
| `corpus/base-t68sites` | 170 | **5.12x / 4.87x** | 2.08x / 2.00x | 1.75x / 1.68x | 1.34x / 1.37x |
| `corpus/base-prepass-t167` | 118 | **4.55x / 4.03x** | 6.88x / 6.02x | 1.64x / 1.46x | 1.45x / 1.49x |
| **`SH_OPEN_TOP_CANON`** | 16 | **3.26x / 2.91x** | 3.00x / 2.73x | 1.70x / 1.65x | 1.27x / 1.46x |
| **`SH_OPEN_BOT_CANON`** | 15 | **3.00x / 2.79x** | 2.89x / 2.73x | 1.73x / 1.54x | 1.42x / 1.41x |
| **`IB_DEEP_POS`** | 24 | **5.19x / 4.49x** | 6.42x / 5.47x | 1.64x / 1.41x | 1.44x / 1.45x |
| **`IB_DEEP_NEG`** | 16 | **12.62x / 11.36x** | 15.97x / 14.41x | 1.49x / 1.36x | 1.51x / 1.47x |

**12-way concurrent load widens the gap in canonical's favor:** under 12-way concurrency, `rust_pnp` degrades **1.52x–1.75x** across `base-t68sites` and `SH_OPEN_*_CANON`, whereas `orca_v2_5_0` degrades only **1.27x–1.46x** — pushing `base-t68sites` from **3.92x → 4.87x–5.12x**, `opcode/offset` from **4.96x → 5.16x–5.78x**, and `SH_OPEN_TOP_CANON` from **2.45–2.57x → 2.91x–3.26x**. Smaller decimated working sets fit better in L2/L3 cache under 12-core saturation.

---

## 5. Mechanism Decomposition: 3-Way Factor Ablation (`ablation_decimation.json`)

To tell the next ticket *which* change in canonical buys the speedup, we built a fifth diagnostic binary `orca_v2_5_0_nodecim` (identical to `orca_v2_5_0` except `CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR = 0.0`) and ran all four (`rust_pnp`, `cpp_c2_1_5_2`, `orca_v2_5_0_nodecim`, `orca_v2_5_0`) forward + reverse (10 timed repeats per case) on the 118 prepass cases:

$$\frac{T(\text{rust\_pnp})}{T(\text{orca\_v2\_5\_0})} = \underbrace{\frac{T(\text{rust\_pnp})}{T(\text{cpp\_c2\_1\_5\_2})}}_{f_1:\;\text{Rust 1.1}\to\text{C++ 1.5.2}} \times \underbrace{\frac{T(\text{cpp\_c2\_1\_5\_2})}{T(\text{orca\_v2\_5\_0\_nodecim})}}_{f_2:\;\text{1.5.2}\to\text{2.0.1 + ClipperUtils (no decim)}} \times \underbrace{\frac{T(\text{orca\_v2\_5\_0\_nodecim})}{T(\text{orca\_v2\_5\_0})}}_{f_3:\;\text{pure edge decimation } 0.005\cdot|\delta|}$$

| Site | Total (`rust / v250`) | `f1` Rust 1.1 → C++ 1.5.2 | `f2` 1.5.2 → 2.0.1 + wrapper (no decim) | `f3` pure edge decimation (`0.005 × \|δ\|`) | med Δarea (no-decim → decim) |
|---|---:|---:|---:|---:|---:|
| **`IB_DEEP_NEG`** (`-1.6069 mm Miter 2`) | **12.54x** | 1.23x | 1.23x | **8.27x** | 0.0047% → 6.443% |
| **`IB_DEEP_POS`** (`+0.5356 mm Miter 2`) | **4.83x** | 1.20x | 1.13x | **3.56x** | 0.0002% → 0.350% |
| **`SH_OPEN_BOT_CANON`** (`0.04 mm Miter 3`) | **3.04x** | 1.40x | **1.72x** | 1.26x | 0.116% → 0.097% |
| **`SH_OPEN_TOP_CANON`** (`0.04 mm Miter 3`) | **3.18x** | 1.41x | **1.76x** | 1.29x | 0.132% → 0.117% |
| **`SH_OPEN_BOT_LEGACY`** (`0.20 mm Round`) | **2.77x** | 1.47x | 0.92x | **2.04x** | 0.0038% → 0.0052% |
| **`SH_OPEN_TOP_LEGACY`** (`0.20 mm Round`) | **2.77x** | 1.34x | 0.98x | **2.10x** | 0.0013% → 0.0037% |
| `IB_CAND_POS` (`+0.3571 mm Miter 2`, 24v) | 1.57x | 1.09x | 1.29x | 1.12x | 0.0017% → 0.0032% |

Two distinct, additive levers emerge:
1. **`append_offset_path` edge decimation (`0.005 × |delta|`) is the dominant lever on large-delta offsets (`f3 = 3.56x–8.27x` on `IB_DEEP_*`, `2.04x–2.10x` on `0.20 mm` openings, and the main driver of the `3.0x–6.1x` speedup on `base-t68sites` offsets).** It is a ~25-line pre-offset ring decimation filter that drops consecutive edges shorter than `0.005 * |delta|` before invoking `ClipperOffset`.
2. **Clipper2 2.0.1 + flat-path `offset2_ex` pass 1 (`f2 = 1.72x–1.76x` on canonical `0.04 mm Miter 3` shell openings even with zero decimation).** Canonical `v2.5.0` `offset2_ex` runs pass 1 (`-r`) into flat `Paths64` (skipping the intermediate `PolyTree64` construction and `ExPolygon` re-flattening that PnP's `offset2_ex` and `driver_cpp_c2.cpp` pay), paired with Clipper2 2.0.1's `ClipperOffset` + `PreserveCollinear(false)` improvements.
3. **Why NOT port Clipper1?** Canonical itself abandoned Clipper1 for Clipper2 2.0.1 (`222c6a2df5`). On our corpus, `orca_v2_5_0` beats `orca_clipper1` on `offset` (`0.78x` sum, `0.47x` median), `base-t68sites` (`0.45x`), `union` (`0.79x`), and canonical `SH_OPEN_*_CANON` (`2.45–2.57x` vs `2.02–2.06x`), AND avoids Clipper1's degenerate-sliver bug (§6).

---

## 6. Geometry-Delta Audit (Why Speed Is Honest, Not Broken)

- **`cpp_c2_1_5_2` vs `rust_pnp`:** exact `0.000000` median area delta across every corpus (`0` empty mismatches).
- **`orca_v2_5_0` vs `rust_pnp`:**
  - Zero empty mismatches (`28/28` empty cases match across all 405 unique cases).
  - Median relative area delta is **0.0044%** across all 405 unique cases (**0.0032%** on `offset`, **0.0365%** on `opening_ex`, **0.0000%** on `union`, **0.0022%** on `intersection`).
  - **The one systematic area delta — `IB_DEEP_NEG` (`+6.44%` median, up to `+49.21%`) — is physically real and explained by decimation:** on `IB_DEEP_NEG` `L=38`, `poly 0` contains a microscopic 5 µm × 5 µm pinhole (`signed_area = -2,541.5` u² = `0.000025 mm²`). Without decimation (`rust_pnp`, `cpp_c2_1_5_2`, `orca_v2_5_0_nodecim` where med Δarea is `0.0047%`), a `-1.6069 mm` erosion inflates that 5 µm pinhole into a 3.2 mm hole (`~10 mm²`) punched through the solid. Canonical's `ShortestEdgeLength = 0.005 × 1.6069 = 8.03 µm` collapses the 5 µm pinhole before offsetting. Because `IB_DEEP_NEG` is an offset with non-zero output-shape impact, adopting edge decimation in PnP is a **non-output-neutral canonical-parity change** (like `t166`), not a byte-identical refactor.
- **`orca_clipper1` degenerate-sliver bug (2 empty mismatches, `max_rel_area_delta = 1.0`):** on `IB_DEEP_NEG` `L=133` and `L=493`, both `rust_pnp` and `orca_v2_5_0` correctly return empty (`0.0`), while `orca_clipper1` returns **10.85 mm²** and **4.81 mm²** solid blobs. Isolation of `L=133` traced this to `P6` (`nv=8`, a 0.1 µm × 240 µm hairline sliver): Clipper1's `ShortestEdgeLength` collapses the 0.1 µm return end into a reversed self-intersecting loop that Clipper1 inflates outward under a negative delta.

---

## 7. Verification & Reproducibility

Every number above is stored in checkout-resident `results/*.json` and re-derived without `.local-artifacts/` or running engines:

```bash
python docs/specs/perf-vs-orca/evidence/t167-canonical-clipperutils-replay/verify.py --self-test
```

`verify.py --self-test` verifies all 11 sealed driver/script SHA-256 hashes, all 9 corpus SHA-256 hashes, prepass census identity against `t166`'s `0a1ee3df…` capture hash, all 6 reduced JSON seals, 100% forward/reverse and single/loaded output determinism across all 4 engines, exact re-derivation of `summary.json` and `ablation_decimation.json`, and rejects **6/6 negative controls** (`changed-timing`, `deleted-case`, `shifted-site`, `output-drift`, `changed-decision`, `tampered-seal`).
